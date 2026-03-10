# linesweeper

Python bindings for [linesweeper](https://github.com/jneem/linesweeper), a Rust library for 2D boolean path operations.

Provides a `union()` function compatible with [ufo2ft](https://github.com/googlefonts/ufo2ft)'s overlap removal filter, as a drop-in replacement for [skia-pathops](https://github.com/fonttools/skia-pathops).

## Install

Requires Rust toolchain and [maturin](https://www.maturin.rs/):

```
pip install maturin
maturin develop
```

## Usage

```python
from linesweeper import union

# contours: list of objects with a .draw(pen) method (e.g. defcon contours)
# outpen: a fontTools segment pen (moveTo/lineTo/curveTo/closePath)
union(contours, outpen, clockwise=True)
```

## API

- **`union(contours, outpen, clockwise=False)`** — merge overlapping contours using nonzero winding fill rule

Set `clockwise=True` for font convention (outer contours clockwise, holes counter-clockwise).

## License

Apache-2.0
