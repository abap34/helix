use crate::compositor::Event;
use helix_view::{
    editor::Action,
    graphics::{Modifier, Rect},
    icons::ICONS,
    input::{KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    keyboard::{KeyCode, KeyModifiers},
    Editor,
};
use ignore::WalkBuilder;
use std::{
    collections::HashSet,
    error::Error,
    io,
    path::{Path, PathBuf},
};
use tui::buffer::Buffer as Surface;

const MIN_SIDEBAR_WIDTH: u16 = 18;
const MAX_SIDEBAR_WIDTH: u16 = 36;
const MIN_EDITOR_WIDTH: u16 = 40;
const HEADER_HEIGHT: u16 = 2;
const INDENT_STEP: usize = 2;

pub(super) enum Interaction {
    Ignored,
    Consumed,
    Close,
}

#[derive(Clone)]
struct VisibleEntry {
    path: PathBuf,
    label: String,
    depth: u16,
    kind: EntryKind,
}

#[derive(Clone, Copy)]
enum EntryKind {
    Directory { expanded: bool },
    File,
}

impl VisibleEntry {
    fn is_dir(&self) -> bool {
        matches!(self.kind, EntryKind::Directory { .. })
    }

    fn is_expanded(&self) -> bool {
        matches!(self.kind, EntryKind::Directory { expanded: true })
    }
}

pub(super) struct FileTree {
    root: PathBuf,
    expanded: HashSet<PathBuf>,
    entries: Vec<VisibleEntry>,
    cursor: usize,
    scroll: usize,
    focused: bool,
    area: Rect,
}

impl FileTree {
    pub(super) fn new(root: PathBuf, editor: &mut Editor) -> io::Result<Self> {
        let mut tree = Self {
            root,
            expanded: HashSet::new(),
            entries: Vec::new(),
            cursor: 0,
            scroll: 0,
            focused: true,
            area: Rect::default(),
        };

        tree.expanded.insert(tree.root.clone());
        tree.focus(editor)?;
        Ok(tree)
    }

    pub(super) fn root(&self) -> &Path {
        &self.root
    }

    pub(super) fn is_focused(&self) -> bool {
        self.focused
    }

    pub(super) fn width_for(&self, available_width: u16) -> u16 {
        let min_width = MIN_SIDEBAR_WIDTH.min(available_width);
        let max_width = available_width
            .saturating_sub(MIN_EDITOR_WIDTH)
            .clamp(min_width, MAX_SIDEBAR_WIDTH.min(available_width));
        let desired = ((available_width as u32) * 28 / 100) as u16;

        desired.clamp(min_width, max_width.max(min_width))
    }

    pub(super) fn focus(&mut self, editor: &mut Editor) -> io::Result<()> {
        self.focused = true;
        editor.enter_normal_mode();
        self.reveal_current_document(editor)
    }

    pub(super) fn handle_event(&mut self, event: &Event, editor: &mut Editor) -> Interaction {
        match event {
            Event::Key(key) => self.handle_key(*key, editor),
            Event::Mouse(mouse) => self.handle_mouse(*mouse, editor),
            Event::Resize(..) => Interaction::Consumed,
            _ => Interaction::Ignored,
        }
    }

    pub(super) fn render(&mut self, area: Rect, surface: &mut Surface, editor: &Editor) {
        self.area = area;
        self.ensure_cursor_visible();

        let panel_style = editor
            .theme
            .try_get("ui.menu")
            .unwrap_or_else(|| editor.theme.get("ui.popup"));
        let header_style = editor
            .theme
            .try_get("ui.picker.header")
            .unwrap_or_else(|| panel_style.add_modifier(Modifier::BOLD));
        let selected_style = editor
            .theme
            .try_get("ui.menu.selected")
            .unwrap_or_else(|| editor.theme.get("ui.text.focus"));
        let directory_style = editor.theme.get("ui.text.directory");
        let text_style = editor.theme.get("ui.text");
        let focus_style = editor.theme.get("ui.text.focus");
        let separator_style = editor.theme.get("ui.background.separator");
        let border_style = if self.focused {
            editor.theme.get("ui.window").patch(
                editor
                    .theme
                    .try_get_exact("ui.window.active")
                    .unwrap_or(focus_style),
            )
        } else {
            editor.theme.get("ui.window")
        };

        if area.width == 0 || area.height == 0 {
            return;
        }

        surface.clear_with(area, panel_style);

        let separator_x = area.right().saturating_sub(1);
        for y in area.top()..area.bottom() {
            if let Some(cell) = surface.get_mut(separator_x, y) {
                cell.set_symbol(tui::symbols::line::VERTICAL)
                    .set_style(border_style);
            }
        }

        let inner = area.clip_right(1);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let header_x = inner.x.saturating_add(1);
        let header_width = inner.width.saturating_sub(2) as usize;

        surface.set_stringn(
            header_x,
            inner.y,
            self.root.display().to_string(),
            header_width,
            if self.focused {
                header_style
            } else {
                panel_style
            },
        );

        let separator_y = inner.y.saturating_add(1);
        if separator_y < inner.bottom() {
            for x in inner.left()..inner.right() {
                if let Some(cell) = surface.get_mut(x, separator_y) {
                    cell.set_symbol(tui::symbols::line::HORIZONTAL)
                        .set_style(separator_style);
                }
            }
        }

        let current_path = current_document_path(editor);
        let visible_rows = self.visible_rows();
        let list_y = inner.y.saturating_add(HEADER_HEIGHT);

        for row in 0..visible_rows {
            let index = self.scroll + row as usize;
            if index >= self.entries.len() {
                break;
            }

            let entry = &self.entries[index];
            let is_selected = index == self.cursor;
            let is_current = current_path
                .as_ref()
                .is_some_and(|path| path == &entry.path);
            let row_y = list_y + row;
            let row_area = Rect::new(inner.x, row_y, inner.width, 1);

            let base_style = if entry.is_dir() {
                directory_style
            } else {
                text_style
            };
            let line_style = if is_selected {
                if self.focused {
                    selected_style.add_modifier(Modifier::BOLD)
                } else {
                    selected_style
                }
            } else if is_current {
                base_style.patch(focus_style.add_modifier(Modifier::BOLD))
            } else {
                base_style
            };

            surface.set_style(row_area, line_style);

            let mut x = inner.x;
            let marker = if is_current { "* " } else { "  " };
            x = surface
                .set_stringn(x, row_y, marker, inner.width as usize, line_style)
                .0;

            let indent = " ".repeat(entry.depth as usize * INDENT_STEP);
            x = surface
                .set_stringn(x, row_y, indent, inner.width as usize, line_style)
                .0;

            let icons = ICONS.load();
            let icon_style = match entry.kind {
                EntryKind::Directory { expanded } => {
                    let icon = icons.fs().directory(expanded).unwrap_or(if expanded {
                        "󰝰"
                    } else {
                        "󰉋"
                    });
                    surface
                        .set_stringn(
                            x,
                            row_y,
                            format!("{icon} "),
                            inner.width as usize,
                            line_style,
                        )
                        .0
                }
                EntryKind::File => {
                    if let Some(icon) = icons
                        .fs()
                        .from_path(&entry.path)
                        .cloned()
                        .or_else(|| icons.kind().file())
                    {
                        let style = icon
                            .color()
                            .map(|color| line_style.fg(color))
                            .unwrap_or(line_style);
                        surface
                            .set_stringn(x, row_y, format!("{icon} "), inner.width as usize, style)
                            .0
                    } else {
                        x
                    }
                }
            };
            let suffix = if entry.is_dir() { "/" } else { "" };
            let remaining_width = inner.right().saturating_sub(icon_style);
            if remaining_width > 0 {
                surface.set_stringn(
                    icon_style,
                    row_y,
                    format!("{}{suffix}", entry.label),
                    remaining_width as usize,
                    line_style,
                );
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent, editor: &mut Editor) -> Interaction {
        if !self.focused {
            return Interaction::Ignored;
        }

        let handled = match (key.code, key.modifiers) {
            (KeyCode::Esc, KeyModifiers::NONE) => {
                self.focused = false;
                true
            }
            (KeyCode::Char('q'), KeyModifiers::NONE) => return Interaction::Close,
            (KeyCode::Up, KeyModifiers::NONE) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                self.move_cursor(-1, editor);
                true
            }
            (KeyCode::Down, KeyModifiers::NONE) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                self.move_cursor(1, editor);
                true
            }
            (KeyCode::PageUp, KeyModifiers::NONE) => {
                self.move_cursor(-(self.visible_rows() as isize).max(1), editor);
                true
            }
            (KeyCode::PageDown, KeyModifiers::NONE) => {
                self.move_cursor((self.visible_rows() as isize).max(1), editor);
                true
            }
            (KeyCode::Home, KeyModifiers::NONE) => {
                self.set_cursor(0, editor);
                true
            }
            (KeyCode::End, KeyModifiers::NONE) => {
                self.set_cursor(self.entries.len().saturating_sub(1), editor);
                true
            }
            (KeyCode::Left, KeyModifiers::NONE) | (KeyCode::Char('h'), KeyModifiers::NONE) => {
                self.handle_left(editor);
                true
            }
            (KeyCode::Right, KeyModifiers::NONE) | (KeyCode::Char('l'), KeyModifiers::NONE) => {
                self.handle_right(editor);
                true
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                self.activate_current(editor);
                true
            }
            _ => false,
        };

        if handled {
            Interaction::Consumed
        } else {
            Interaction::Ignored
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, editor: &mut Editor) -> Interaction {
        let inside = self.contains(mouse.row, mouse.column);

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) if inside => {
                self.focused = true;
                editor.enter_normal_mode();

                if let Some(index) = self.index_at_row(mouse.row) {
                    self.set_cursor(index, editor);
                }

                Interaction::Consumed
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if self.focused {
                    self.focused = false;
                }
                Interaction::Ignored
            }
            _ => Interaction::Ignored,
        }
    }

    fn handle_left(&mut self, editor: &mut Editor) {
        if self.entries.is_empty() {
            return;
        }

        if self.entries[self.cursor].is_dir() && self.entries[self.cursor].is_expanded() {
            self.expanded.remove(&self.entries[self.cursor].path);
            if let Err(err) = self.rebuild(editor) {
                editor.set_error(err.to_string());
            }
            return;
        }

        if let Some(parent) = self.parent_index(self.cursor) {
            self.set_cursor(parent, editor);
        }
    }

    fn handle_right(&mut self, editor: &mut Editor) {
        if self.entries.is_empty() || !self.entries[self.cursor].is_dir() {
            return;
        }

        if !self.entries[self.cursor].is_expanded() {
            self.expanded.insert(self.entries[self.cursor].path.clone());
            if let Err(err) = self.rebuild(editor) {
                self.expanded.remove(&self.entries[self.cursor].path);
                editor.set_error(err.to_string());
            }
            return;
        }

        let child_index = self.cursor + 1;
        if child_index < self.entries.len()
            && self.entries[child_index].depth > self.entries[self.cursor].depth
        {
            self.set_cursor(child_index, editor);
        }
    }

    fn activate_current(&mut self, editor: &mut Editor) {
        let Some(entry) = self.entries.get(self.cursor).cloned() else {
            return;
        };

        if entry.is_dir() {
            if entry.is_expanded() {
                self.expanded.remove(&entry.path);
            } else {
                self.expanded.insert(entry.path.clone());
            }

            if let Err(err) = self.rebuild(editor) {
                if entry.is_expanded() {
                    self.expanded.insert(entry.path);
                } else {
                    self.expanded.remove(&entry.path);
                }
                editor.set_error(err.to_string());
            }
            return;
        }

        if let Err(err) = editor.open(&entry.path, Action::Replace) {
            let err = match err.source() {
                Some(err) => err.to_string(),
                None => format!("unable to open \"{}\"", entry.path.display()),
            };
            editor.set_error(err);
            return;
        }

        self.focused = false;
        let _ = self.reveal_current_document(editor);
    }

    fn move_cursor(&mut self, delta: isize, editor: &mut Editor) {
        if self.entries.is_empty() {
            self.cursor = 0;
            self.scroll = 0;
            return;
        }

        let max_index = self.entries.len().saturating_sub(1) as isize;
        let next = (self.cursor as isize + delta).clamp(0, max_index) as usize;
        self.set_cursor(next, editor);
    }

    fn set_cursor(&mut self, index: usize, editor: &mut Editor) {
        self.cursor = index.min(self.entries.len().saturating_sub(1));
        self.ensure_cursor_visible();
        self.preview_current(editor);
    }

    fn preview_current(&mut self, editor: &mut Editor) {
        let Some(entry) = self.entries.get(self.cursor) else {
            return;
        };

        if entry.is_dir() {
            return;
        }

        if current_document_path(editor)
            .as_ref()
            .is_some_and(|path| path == &entry.path)
        {
            return;
        }

        if let Err(err) = editor.open(&entry.path, Action::Replace) {
            let err = match err.source() {
                Some(err) => err.to_string(),
                None => format!("unable to open \"{}\"", entry.path.display()),
            };
            editor.set_error(err);
        }
    }

    fn reveal_current_document(&mut self, editor: &Editor) -> io::Result<()> {
        let current_path = current_document_path(editor);

        if let Some(path) = current_path
            .as_ref()
            .filter(|path| path.starts_with(&self.root))
        {
            let mut ancestor = if path.is_dir() {
                Some(path.as_path())
            } else {
                path.parent()
            };

            while let Some(dir) = ancestor {
                if !dir.starts_with(&self.root) {
                    break;
                }
                self.expanded.insert(dir.to_path_buf());
                if dir == self.root.as_path() {
                    break;
                }
                ancestor = dir.parent();
            }
        }

        self.rebuild(editor)?;

        if let Some(path) = current_path {
            if let Some(index) = self.entries.iter().position(|entry| entry.path == path) {
                self.cursor = index;
            }
        }

        self.ensure_cursor_visible();
        Ok(())
    }

    fn rebuild(&mut self, editor: &Editor) -> io::Result<()> {
        let selected_path = self
            .entries
            .get(self.cursor)
            .map(|entry| entry.path.clone());
        let root_expanded = self.expanded.contains(&self.root);
        let mut entries = vec![VisibleEntry {
            path: self.root.clone(),
            label: display_name(&self.root),
            depth: 0,
            kind: EntryKind::Directory {
                expanded: root_expanded,
            },
        }];

        if root_expanded {
            self.append_directory_entries(&self.root, 1, editor, &mut entries)?;
        }

        self.entries = entries;
        if let Some(path) = selected_path {
            if let Some(index) = self.entries.iter().position(|entry| entry.path == path) {
                self.cursor = index;
            } else {
                self.cursor = self.cursor.min(self.entries.len().saturating_sub(1));
            }
        } else {
            self.cursor = self.cursor.min(self.entries.len().saturating_sub(1));
        }

        self.ensure_cursor_visible();
        Ok(())
    }

    fn append_directory_entries(
        &self,
        directory: &Path,
        depth: u16,
        editor: &Editor,
        entries: &mut Vec<VisibleEntry>,
    ) -> io::Result<()> {
        for (path, is_dir) in read_directory_entries(directory, editor)? {
            let expanded = is_dir && self.expanded.contains(&path);
            entries.push(VisibleEntry {
                label: display_name(&path),
                depth,
                kind: if is_dir {
                    EntryKind::Directory { expanded }
                } else {
                    EntryKind::File
                },
                path: path.clone(),
            });

            if is_dir && expanded {
                self.append_directory_entries(&path, depth + 1, editor, entries)?;
            }
        }

        Ok(())
    }

    fn contains(&self, row: u16, column: u16) -> bool {
        row >= self.area.top()
            && row < self.area.bottom()
            && column >= self.area.left()
            && column < self.area.right()
    }

    fn visible_rows(&self) -> u16 {
        self.area.height.saturating_sub(HEADER_HEIGHT)
    }

    fn ensure_cursor_visible(&mut self) {
        let visible_rows = self.visible_rows() as usize;
        if visible_rows == 0 {
            self.scroll = 0;
            return;
        }

        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        } else if self.cursor >= self.scroll + visible_rows {
            self.scroll = self.cursor + 1 - visible_rows;
        }
    }

    fn index_at_row(&self, row: u16) -> Option<usize> {
        let list_top = self.area.y.saturating_add(HEADER_HEIGHT);
        if row < list_top || row >= self.area.bottom() {
            return None;
        }

        let index = self.scroll + (row - list_top) as usize;
        (index < self.entries.len()).then_some(index)
    }

    fn parent_index(&self, index: usize) -> Option<usize> {
        let depth = self.entries.get(index)?.depth;
        if depth == 0 {
            return None;
        }

        (0..index)
            .rev()
            .find(|candidate| self.entries[*candidate].depth == depth - 1)
    }
}

fn current_document_path(editor: &Editor) -> Option<PathBuf> {
    let view = editor.tree.get(editor.tree.focus);
    let doc = editor.document(view.doc)?;
    doc.path().map(helix_stdx::path::canonicalize)
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn read_directory_entries(root: &Path, editor: &Editor) -> io::Result<Vec<(PathBuf, bool)>> {
    let config = editor.config();
    let mut walk_builder = WalkBuilder::new(root);

    let mut entries: Vec<(PathBuf, bool)> = walk_builder
        .hidden(config.file_explorer.hidden)
        .parents(config.file_explorer.parents)
        .ignore(config.file_explorer.ignore)
        .follow_links(config.file_explorer.follow_symlinks)
        .git_ignore(config.file_explorer.git_ignore)
        .git_global(config.file_explorer.git_global)
        .git_exclude(config.file_explorer.git_exclude)
        .max_depth(Some(1))
        .add_custom_ignore_filename(helix_loader::config_dir().join("ignore"))
        .add_custom_ignore_filename(".helix/ignore")
        .types(crate::ui::get_excluded_types())
        .build()
        .filter_map(|entry| {
            entry
                .map(|entry| {
                    let path = helix_stdx::path::canonicalize(entry.path());
                    (path.clone(), path.is_dir())
                })
                .ok()
                .filter(|entry| entry.0 != root)
        })
        .collect();

    entries.sort_by(|(path1, is_dir1), (path2, is_dir2)| (!is_dir1, path1).cmp(&(!is_dir2, path2)));

    Ok(entries)
}
