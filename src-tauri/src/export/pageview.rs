//! The in-editor Page view (SPEC §12.2): the chapter being written, typeset
//! with a PDF profile, and a click on a page mapped back to the text block
//! it came from. Read-only: nothing here changes the manuscript.

use typst::WorldExt;
use typst::layout::{Abs, Frame, FrameItem, Point};
use typst_layout::PagedDocument;

use super::world::BookWorld;

/// A text block in a document, as `story.search` and the editor number them.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PageHit {
    pub doc: String,
    pub block: u32,
}

/// A box on a page holding a run of glyphs that came from `offset` in the
/// generated source.
struct Spot {
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
    offset: usize,
}

fn collect(frame: &Frame, origin: Point, world: &BookWorld, out: &mut Vec<Spot>) {
    for (pos, item) in frame.items() {
        let at = Point::new(origin.x + pos.x, origin.y + pos.y);
        match item {
            FrameItem::Group(g) => {
                let o = Point::new(at.x + g.transform.tx, at.y + g.transform.ty);
                collect(&g.frame, o, world, out);
            }
            FrameItem::Text(t) => {
                let size = t.size;
                let mut x = at.x;
                for glyph in &t.glyphs {
                    let w = glyph.x_advance.at(size);
                    let (span, _) = glyph.span;
                    if let Some(range) = world.range(span) {
                        out.push(Spot {
                            x0: x.to_pt(),
                            x1: (x + w).to_pt(),
                            // y grows downward from the top of the page; the
                            // baseline is at `at.y`.
                            y0: (at.y - size * 0.8).to_pt(),
                            y1: (at.y + size * 0.25).to_pt(),
                            offset: range.start,
                        });
                    }
                    x += w;
                }
            }
            _ => {}
        }
    }
}

/// How far (pt) from any glyph a click may land and still count.
const REACH_PT: f64 = 28.0;

/// The generated-source offset of the glyph nearest to a click on a page.
pub fn offset_at(
    doc: &PagedDocument,
    world: &BookWorld,
    page: usize,
    x: f64,
    y: f64,
) -> Option<usize> {
    let page = doc.pages().get(page)?;
    let mut spots = Vec::new();
    collect(
        &page.frame,
        Point::new(Abs::zero(), Abs::zero()),
        world,
        &mut spots,
    );
    spots
        .iter()
        .map(|s| {
            let dx = (s.x0 - x).max(x - s.x1).max(0.0);
            let dy = (s.y0 - y).max(y - s.y1).max(0.0);
            (dx.hypot(dy), s.offset)
        })
        .filter(|(d, _)| *d <= REACH_PT)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, o)| o)
}

/// The text block whose marker most recently precedes `offset`.
pub fn block_at(map: &[(usize, Option<(String, u32)>)], offset: usize) -> Option<PageHit> {
    let i = map.partition_point(|(end, _)| *end <= offset);
    let (_, target) = map.get(i.checked_sub(1)?)?;
    let (doc, block) = target.as_ref()?;
    Some(PageHit {
        doc: doc.clone(),
        block: *block,
    })
}
