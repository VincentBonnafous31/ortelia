//! The layout of the window, drawn by egui_tiles: a `Linear` container per split, a pane per
//! view. The structure stays that of `app.json`: no tab, no drag and drop, no simplification of
//! the tree; only the separators move, and each move goes back to the presentation as an intent.
//!
//! The tree of egui_tiles is to the layout of the presentation what a tree of SWT widgets is to
//! the EMF model it shows: the truth is in the model (the presentation), the widgets report the
//! gestures of the user. It is built once, at the opening, after the session state was restored;
//! from then on, only the user changes the shares, and the presentation follows by the intents.

use egui_tiles::{
    Behavior, Container, EditAction, Linear, LinearDir, SimplificationOptions, Tile, TileId, Tiles,
    Tree, UiResponse,
};
use ortelia_presentation::{AppState, Direction, Intent, LayoutNode, MainContent, ViewRef};

use crate::{form, table};

/// The margin of a view, on each side, in points: that of the central panel of egui, which has
/// none around the tiles, so that they reach the edges of the window. The widgets of a view do
/// not touch the separators.
pub(crate) const VIEW_MARGIN: i8 = 8;

/// A difference of share below this is a rounding, not a move of a separator: a point in a
/// window of 4000 points is 0.00025.
const SHARE_TOLERANCE: f32 = 1e-5;

/// A pane of egui_tiles: a view, by its identifier.
#[derive(Debug)]
struct Pane {
    view: String,
}

/// The tree of egui_tiles of the layout of an application, and the tile of each split.
pub(crate) struct LayoutTiles {
    tree: Tree<Pane>,
    /// The splits, by identifier, with their tile, in the order of the layout.
    splits: Vec<(String, TileId)>,
}

impl LayoutTiles {
    /// The tiles of the layout of `state`; `None` without application.
    pub(crate) fn of(state: &AppState) -> Option<LayoutTiles> {
        let MainContent::Layout(layout) = state.content() else {
            return None;
        };
        let mut tiles = Tiles::default();
        let mut splits = Vec::new();
        let root = insert(&mut tiles, layout, &mut splits);
        Some(LayoutTiles {
            tree: Tree::new(egui::Id::unique("ortelia-layout"), root, tiles),
            splits,
        })
    }

    /// Draws the layout in `ui`, the views from `state`. Gives the intents of the views, then
    /// the shares of each split whose separator moved.
    pub(crate) fn draw(&mut self, ui: &mut egui::Ui, state: &AppState) -> Vec<Intent> {
        let mut behavior = LayoutBehavior {
            state,
            intents: Vec::new(),
            resized: false,
        };
        self.tree.ui(&mut behavior, ui);
        let mut intents = behavior.intents;
        // Only after a move by the user: a change of the parts by the presentation is never
        // sent back to it.
        if !behavior.resized {
            return intents;
        }
        let MainContent::Layout(layout) = state.content() else {
            return intents;
        };
        for (id, tile) in &self.splits {
            if let Some(Tile::Container(Container::Linear(linear))) = self.tree.tiles.get(*tile)
                && let Some(split) = layout.split(id)
            {
                let shares: Vec<f32> = linear.children.iter().map(|&c| linear.shares[c]).collect();
                // egui_tiles signals a resize at each frame while the separator is held, moved
                // or not: only shares that differ from the parts of the presentation are given,
                // as for the widths of the columns, so that nothing is drawn again for nothing.
                let total: f32 = shares.iter().sum();
                let moved = shares
                    .iter()
                    .zip(split.parts())
                    .any(|(share, part)| (share / total - part).abs() > SHARE_TOLERANCE);
                if moved {
                    intents.push(Intent::SplitResized {
                        split: id.clone(),
                        shares,
                    });
                }
            }
        }
        intents
    }
}

/// Inserts the tiles of `node` and of its children; gives the tile of `node`.
fn insert(
    tiles: &mut Tiles<Pane>,
    node: &LayoutNode,
    splits: &mut Vec<(String, TileId)>,
) -> TileId {
    match node {
        LayoutNode::View(view) => tiles.insert_pane(Pane {
            view: view.id().to_owned(),
        }),
        LayoutNode::Split(split) => {
            let children: Vec<TileId> = split
                .children()
                .iter()
                .map(|child| insert(tiles, child, splits))
                .collect();
            let dir = match split.direction() {
                Direction::Horizontal => LinearDir::Horizontal,
                Direction::Vertical => LinearDir::Vertical,
            };
            let mut linear = Linear::new(dir, children);
            // Only the ratios count. egui_tiles gives 1 to each child by default: the same
            // total, the number of children, keeps its double-click (which averages two
            // neighbours) consistent.
            let count = linear.children.len() as f32;
            for (&child, &part) in linear.children.iter().zip(split.parts()) {
                linear.shares.set_share(child, part * count);
            }
            let tile = tiles.insert_container(linear);
            splits.push((split.id().to_owned(), tile));
            tile
        }
    }
}

/// How egui_tiles draws our tiles, for one frame: it borrows the state, to draw the views, and
/// gathers their intents.
struct LayoutBehavior<'a> {
    state: &'a AppState,
    intents: Vec<Intent>,
    /// Set when a separator moved, by a drag or a double-click.
    resized: bool,
}

impl Behavior<Pane> for LayoutBehavior<'_> {
    fn pane_ui(&mut self, ui: &mut egui::Ui, _tile_id: TileId, pane: &mut Pane) -> UiResponse {
        egui::Frame::new().inner_margin(VIEW_MARGIN).show(ui, |ui| {
            match self.state.view(&pane.view) {
                // The intents of the table, addressed to its view.
                Some(ViewRef::Table(table)) => self.intents.extend(
                    table::draw_table_view(ui, table)
                        .into_iter()
                        .map(|intent| Intent::table(&pane.view, intent)),
                ),
                // A form emits no intent: its widgets cannot change a value.
                Some(ViewRef::Form(view)) => form::draw_form_view(ui, view),
                // An empty view draws nothing.
                Some(ViewRef::Empty) | None => {}
            }
        });
        // Never `DragStarted`: a view is not moved.
        UiResponse::None
    }

    /// Required by the trait; never shown, there being no tab.
    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        pane.view.clone().into()
    }

    /// Neither a view nor a split can be dragged: the structure is that of `app.json`.
    fn is_tile_draggable(&self, _tiles: &Tiles<Pane>, _tile_id: TileId) -> bool {
        false
    }

    /// No simplification: by default, egui_tiles would join two nested splits of the same
    /// direction, or remove a split left with one child, and the tiles would no longer match
    /// the splits of the presentation.
    fn simplification_options(&self) -> SimplificationOptions {
        SimplificationOptions::OFF
    }

    fn on_edit(&mut self, action: EditAction) {
        if action == EditAction::TileResized {
            self.resized = true;
        }
    }
}

#[cfg(test)]
mod tests;
