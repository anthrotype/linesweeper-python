#![allow(non_snake_case)]

use kurbo::{BezPath, PathEl};
use linesweeper::topology::Topology;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyTuple;

/// A pen that builds a kurbo::BezPath from fontTools segment pen calls.
#[pyclass]
struct BezPathPen {
    path: BezPath,
}

#[pymethods]
impl BezPathPen {
    fn moveTo(&mut self, pt: (f64, f64)) {
        self.path.move_to(pt);
    }

    fn lineTo(&mut self, pt: (f64, f64)) {
        self.path.line_to(pt);
    }

    #[pyo3(signature = (*points))]
    fn curveTo(&mut self, points: &Bound<'_, PyTuple>) -> PyResult<()> {
        let n = points.len();
        match n {
            1 => {
                let pt: (f64, f64) = points.get_item(0)?.extract()?;
                self.path.line_to(pt);
            }
            3 => {
                let pt1: (f64, f64) = points.get_item(0)?.extract()?;
                let pt2: (f64, f64) = points.get_item(1)?.extract()?;
                let pt3: (f64, f64) = points.get_item(2)?.extract()?;
                self.path.curve_to(pt1, pt2, pt3);
            }
            _ => {
                return Err(PyValueError::new_err(format!(
                    "curveTo requires 1-3 points, got {n}"
                )));
            }
        }
        Ok(())
    }

    #[pyo3(signature = (*_points))]
    fn qCurveTo(&mut self, _points: &Bound<'_, PyTuple>) -> PyResult<()> {
        Err(PyValueError::new_err(
            "Quadratic curves are not supported; convert to cubic first",
        ))
    }

    fn closePath(&mut self) {
        self.path.close_path();
    }

    fn endPath(&mut self) {
        // open paths: just ignore (linesweeper requires closed paths)
    }

    #[pyo3(signature = (_glyph_name, _transformation))]
    fn addComponent(&self, _glyph_name: &str, _transformation: PyObject) -> PyResult<()> {
        Err(PyValueError::new_err(
            "Components must be decomposed before overlap removal",
        ))
    }
}

/// Draw a kurbo BezPath to a Python pen object using the segment pen protocol.
fn draw_to_pen(py: Python<'_>, path: &BezPath, pen: &Bound<'_, PyAny>) -> PyResult<()> {
    let mut last_pt = kurbo::Point::ZERO;
    for el in path.elements() {
        match el {
            PathEl::MoveTo(p) => {
                last_pt = *p;
                pen.call_method1("moveTo", ((p.x, p.y),))?;
            }
            PathEl::LineTo(p) => {
                last_pt = *p;
                pen.call_method1("lineTo", ((p.x, p.y),))?;
            }
            PathEl::QuadTo(p1, p2) => {
                // Degree-elevate quad to cubic: cubic control points are
                // (1/3 * start + 2/3 * ctrl, 2/3 * ctrl + 1/3 * end)
                let p0 = last_pt;
                let c1 = (
                    p0.x + (2.0 / 3.0) * (p1.x - p0.x),
                    p0.y + (2.0 / 3.0) * (p1.y - p0.y),
                );
                let c2 = (
                    p2.x + (2.0 / 3.0) * (p1.x - p2.x),
                    p2.y + (2.0 / 3.0) * (p1.y - p2.y),
                );
                let args = PyTuple::new(py, [c1, (c2.0, c2.1), (p2.x, p2.y)])?;
                pen.call_method1("curveTo", args)?;
            }
            PathEl::CurveTo(p1, p2, p3) => {
                last_pt = *p3;
                let args =
                    PyTuple::new(py, [(p1.x, p1.y), (p2.x, p2.y), (p3.x, p3.y)])?;
                pen.call_method1("curveTo", args)?;
            }
            PathEl::ClosePath => {
                pen.call_method0("closePath")?;
            }
        }
    }
    Ok(())
}

/// Collect all contours into a single BezPath by drawing them into a
/// BezPathPen via the fontTools segment pen protocol.
fn collect_contours(py: Python<'_>, contours: &[Bound<'_, PyAny>]) -> PyResult<BezPath> {
    let pen = Bound::new(
        py,
        BezPathPen {
            path: BezPath::new(),
        },
    )?;

    for contour in contours {
        contour.call_method1("draw", (&pen,))?;
    }

    let combined = pen.borrow().path.clone();
    Ok(combined)
}

/// Run linesweeper topology on a combined path and draw results to outpen.
fn run_simplify(
    py: Python<'_>,
    combined: &BezPath,
    outpen: &Bound<'_, PyAny>,
    inside: impl Fn(&i32) -> bool,
    clockwise: bool,
) -> PyResult<()> {
    let topology = Topology::from_path(combined, auto_eps(combined))
        .map_err(|e| PyValueError::new_err(format!("linesweeper error: {e}")))?;

    let result = topology.contours(inside);

    // Linesweeper outputs outer contours CCW, holes CW.
    // Font convention (clockwise=true): outer CW, holes CCW — i.e. reversed.
    // When clockwise=false (default): keep linesweeper's native orientation.
    for contour in result.contours() {
        let path = if clockwise {
            contour.path.reverse_subpaths()
        } else {
            contour.path.clone()
        };
        draw_to_pen(py, &path, outpen)?;
    }

    Ok(())
}

/// Remove overlaps from a list of contours by computing their union.
///
/// Each contour must support the fontTools segment pen protocol (i.e. have a
/// `.draw(pen)` method). The result is drawn into `outpen`.
///
/// This is a drop-in replacement for `pathops.union()` as used by ufo2ft.
#[pyfunction]
#[pyo3(signature = (contours, outpen, clockwise=false))]
fn union(
    py: Python<'_>,
    contours: Vec<Bound<'_, PyAny>>,
    outpen: Bound<'_, PyAny>,
    clockwise: bool,
) -> PyResult<()> {
    if contours.is_empty() {
        return Ok(());
    }

    let combined = collect_contours(py, &contours)?;
    if combined.elements().is_empty() {
        return Ok(());
    }

    run_simplify(py, &combined, &outpen, |w| *w != 0, clockwise)
}

/// Compute a reasonable epsilon from path bounding box, same logic as
/// linesweeper's `binary_op`.
fn auto_eps(path: &BezPath) -> f64 {
    use kurbo::Shape;
    let bbox = path.bounding_box();
    let m = bbox
        .min_x()
        .abs()
        .max(bbox.min_y().abs())
        .max(bbox.max_x().abs())
        .max(bbox.max_y().abs());
    (m * f64::EPSILON * 64.0).max(1e-6)
}

pyo3::create_exception!(linesweeper, LinesweeperError, pyo3::exceptions::PyException);

#[pymodule]
fn _linesweeper(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(union, m)?)?;
    m.add("LinesweeperError", m.py().get_type::<LinesweeperError>())?;
    Ok(())
}
