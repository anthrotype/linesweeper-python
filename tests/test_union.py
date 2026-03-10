"""Test linesweeper.union as a drop-in for pathops.union in ufo2ft."""

import ufoLib2
from linesweeper import union


def _new_glyph(name="test", width=600):
    font = ufoLib2.Font()
    glyph = font.newGlyph(name)
    glyph.width = width
    return glyph


def remove_overlaps(glyph):
    """Apply union the same way ufo2ft's RemoveOverlapsFilter does."""
    contours = list(glyph)
    glyph.clearContours()
    union(contours, glyph.getPen(), clockwise=True)


def test_empty():
    glyph = _new_glyph()
    remove_overlaps(glyph)
    assert len(glyph) == 0


def test_single_contour_passthrough():
    glyph = _new_glyph()
    pen = glyph.getPen()
    pen.moveTo((0, 0))
    pen.lineTo((500, 0))
    pen.lineTo((500, 500))
    pen.lineTo((0, 500))
    pen.closePath()

    remove_overlaps(glyph)

    assert len(glyph) == 1
    assert len(glyph.contours[0].points) == 4


def test_overlapping_rects():
    glyph = _new_glyph()
    pen = glyph.getPen()
    pen.moveTo((0, 0))
    pen.lineTo((400, 0))
    pen.lineTo((400, 700))
    pen.lineTo((0, 700))
    pen.closePath()
    pen.moveTo((200, 100))
    pen.lineTo((600, 100))
    pen.lineTo((600, 600))
    pen.lineTo((200, 600))
    pen.closePath()

    remove_overlaps(glyph)

    assert len(glyph) == 1
    assert len(glyph.contours[0].points) == 8


def test_non_overlapping_rects():
    glyph = _new_glyph()
    pen = glyph.getPen()
    pen.moveTo((0, 0))
    pen.lineTo((100, 0))
    pen.lineTo((100, 100))
    pen.lineTo((0, 100))
    pen.closePath()
    pen.moveTo((200, 0))
    pen.lineTo((300, 0))
    pen.lineTo((300, 100))
    pen.lineTo((200, 100))
    pen.closePath()

    remove_overlaps(glyph)

    assert len(glyph) == 2


def test_same_winding_nested():
    """Inner contour wound same direction as outer — nonzero fill merges them."""
    glyph = _new_glyph()
    pen = glyph.getPen()
    pen.moveTo((0, 0))
    pen.lineTo((200, 0))
    pen.lineTo((200, 200))
    pen.lineTo((0, 200))
    pen.closePath()
    pen.moveTo((50, 50))
    pen.lineTo((150, 50))
    pen.lineTo((150, 150))
    pen.lineTo((50, 150))
    pen.closePath()

    remove_overlaps(glyph)

    assert len(glyph) == 1


def test_overlapping_cubics():
    """Two overlapping circles approximated with cubic beziers."""
    glyph = _new_glyph("o", width=500)
    pen = glyph.getPen()
    # Larger circle: center (250,250), radius 250
    pen.moveTo((250, 0))
    pen.curveTo((388, 0), (500, 112), (500, 250))
    pen.curveTo((500, 388), (388, 500), (250, 500))
    pen.curveTo((112, 500), (0, 388), (0, 250))
    pen.curveTo((0, 112), (112, 0), (250, 0))
    pen.closePath()
    # Smaller circle: center (350,250), radius 150
    pen.moveTo((350, 100))
    pen.curveTo((433, 100), (500, 167), (500, 250))
    pen.curveTo((500, 333), (433, 400), (350, 400))
    pen.curveTo((267, 400), (200, 333), (200, 250))
    pen.curveTo((200, 167), (267, 100), (350, 100))
    pen.closePath()

    remove_overlaps(glyph)

    assert len(glyph) == 1