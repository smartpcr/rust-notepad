use eframe::egui::{self, text::LayoutJob, Align2, FontId, TextEdit, Vec2};
use egui_extras::syntax_highlighting;
use rust_notepad::settings::CursorPosition;

use super::RustNotepadApp;

/// Fixed Id for the main editor TextEdit so we can manipulate its state externally.
fn editor_id() -> egui::Id {
    egui::Id::new("codeedit_main_editor")
}

pub fn render(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    let syntax = app.editor.active_doc().syntax.clone();
    let theme = app.app_theme.code_theme(app.view.font_size);
    let font_size = app.view.font_size;
    let word_wrap = app.view.word_wrap;
    let show_line_numbers = app.view.show_line_numbers;
    let show_whitespace = app.view.show_whitespace;

    let ws_color = if ui.visuals().dark_mode {
        egui::Color32::from_gray(90)
    } else {
        egui::Color32::from_gray(180)
    };

    let mut layouter = |ui: &egui::Ui, string: &dyn egui::TextBuffer, wrap_width: f32| {
        let mut job: LayoutJob =
            syntax_highlighting::highlight(ui.ctx(), ui.style(), &theme, string.as_str(), &syntax);
        job.wrap.max_width = if word_wrap { wrap_width } else { f32::INFINITY };
        let mono = FontId::monospace(font_size);
        for section in &mut job.sections {
            section.format.font_id = mono.clone();
        }
        ui.fonts_mut(|f| f.layout_job(job))
    };

    // Handle Ctrl+Scroll zoom
    let scroll_delta = ui.input(|i| {
        if i.modifiers.ctrl {
            i.raw_scroll_delta.y
        } else {
            0.0
        }
    });
    if scroll_delta > 0.0 {
        app.view.zoom_in();
    } else if scroll_delta < 0.0 {
        app.view.zoom_out();
    }

    // Grab pending navigation
    let pending_nav = app.find_state.navigate_to.take();
    let focus_editor = app.find_state.focus_editor;
    app.find_state.focus_editor = false;

    // -- Update fold regions --
    let content_for_folds = app.editor.active_doc().content.clone();
    let syntax_for_folds = app.editor.active_doc().syntax.clone();
    app.editor
        .active_doc_mut()
        .fold_state
        .update_regions(&content_for_folds, &syntax_for_folds);

    // Build display content (with folded lines omitted)
    let has_folds = !app.editor.active_doc().fold_state.collapsed.is_empty();
    let (mut display_content, line_map) = app
        .editor
        .active_doc()
        .fold_state
        .build_display(&content_for_folds);

    // Gutter width calculation
    let total_real_lines = content_for_folds.lines().count().max(1);
    let digit_count = format!("{}", total_real_lines).len().max(2);
    let has_fold_regions = !app.editor.active_doc().fold_state.regions().is_empty();
    let fold_marker_width = if has_fold_regions {
        font_size * 1.2
    } else {
        0.0
    };
    let gutter_width = if show_line_numbers || has_fold_regions {
        let num_width = if show_line_numbers {
            (digit_count as f32 + 1.0) * font_size * 0.6
        } else {
            0.0
        };
        num_width + fold_marker_width + 4.0
    } else {
        0.0
    };

    let scroll_area = egui::ScrollArea::both().auto_shrink([false, false]);

    let output = scroll_area.show(ui, |ui| {
        ui.horizontal_top(|ui| {
            // Reserve gutter space (painted AFTER TextEdit so we can use galley positions)
            let gutter_space = if gutter_width > 0.0 {
                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(gutter_width, ui.available_height()),
                    egui::Sense::click(),
                );
                Some((rect, response))
            } else {
                None
            };

            // Pre-focus
            if pending_nav.is_some() || focus_editor {
                ui.memory_mut(|mem| mem.request_focus(editor_id()));
            }

            // -- Text editor --
            let mut editor_output = if has_folds {
                let output = TextEdit::multiline(&mut display_content)
                    .id(editor_id())
                    .font(FontId::monospace(font_size))
                    .desired_width(f32::INFINITY)
                    .lock_focus(true)
                    .layouter(&mut layouter)
                    .code_editor()
                    .show(ui);
                // If user edited while folded, unfold and apply
                let original_display = app
                    .editor
                    .active_doc()
                    .fold_state
                    .build_display(&app.editor.active_doc().content)
                    .0;
                if display_content != original_display {
                    app.editor.active_doc_mut().fold_state.unfold_all();
                    app.editor.active_doc_mut().content = display_content.clone();
                }
                output
            } else {
                TextEdit::multiline(&mut app.editor.active_doc_mut().content)
                    .id(editor_id())
                    .font(FontId::monospace(font_size))
                    .desired_width(f32::INFINITY)
                    .lock_focus(true)
                    .layouter(&mut layouter)
                    .code_editor()
                    .show(ui)
            };

            // -- Navigate to position --
            if let Some((start, end)) = pending_nav {
                let mut state = editor_output.state.clone();
                let ccursor_range = egui::text::CCursorRange::two(
                    egui::text::CCursor::new(start),
                    egui::text::CCursor::new(end),
                );
                state.cursor.set_char_range(Some(ccursor_range));
                state.store(ui.ctx(), editor_output.response.id);
                editor_output.cursor_range = Some(egui::text::CCursorRange {
                    primary: egui::text::CCursor::new(end),
                    secondary: egui::text::CCursor::new(start),
                    h_pos: None,
                });
                ui.ctx().request_repaint();
            }

            // -- Paint gutter AFTER TextEdit using galley row positions --
            if let Some((gutter_rect, gutter_response)) = gutter_space {
                paint_gutter(
                    ui,
                    app,
                    &editor_output,
                    &line_map,
                    gutter_rect,
                    &gutter_response,
                    &GutterParams {
                        font_size,
                        digit_count,
                        show_line_numbers,
                        fold_marker_width,
                    },
                );
            }

            // -- Whitespace overlay --
            if show_whitespace {
                let content_ref = if has_folds {
                    &display_content
                } else {
                    &app.editor.active_doc().content
                };
                paint_whitespace_markers(ui, &editor_output, content_ref, font_size, ws_color);
            }

            // -- Current line highlight --
            highlight_current_line(ui, &editor_output, &app.app_theme);

            // -- Highlight selected word occurrences --
            let content_ref = if has_folds {
                &display_content
            } else {
                &app.editor.active_doc().content
            };
            highlight_word_occurrences(ui, &editor_output, content_ref);

            // -- Brace matching --
            highlight_matching_brace(ui, &editor_output, content_ref, &app.app_theme);

            // -- XML tag matching --
            let is_xml_like = matches!(
                app.editor.active_doc().syntax.as_str(),
                "xml"
                    | "html"
                    | "htm"
                    | "svg"
                    | "xaml"
                    | "xsl"
                    | "xslt"
                    | "xsd"
                    | "jsp"
                    | "vue"
                    | "svelte"
            );
            if is_xml_like {
                highlight_matching_tag(ui, &editor_output, content_ref);
            }

            editor_output
        })
        .inner
    });

    let editor_output = output.inner;

    // Extract cursor position for status bar.
    if let Some(cursor_range) = editor_output.cursor_range {
        let text = &app.editor.active_doc().content;
        let byte_offset = cursor_range.primary.index;
        let clamped = byte_offset.min(text.len());

        let text_before = &text[..clamped];
        let line = text_before.chars().filter(|&c| c == '\n').count() + 1;
        let col = match text_before.rfind('\n') {
            Some(pos) => clamped - pos,
            None => clamped + 1,
        };

        let selection_len = {
            let a = cursor_range.primary.index;
            let b = cursor_range.secondary.index;
            a.abs_diff(b)
        };

        app.view.cursor = CursorPosition {
            line,
            col,
            byte_offset: clamped,
            selection_len,
        };

        // -- Auto-indent on newline --
        if app.view.auto_indent && !has_folds {
            let content_len = app.editor.active_doc().content.len();
            let prev_len = app.view.prev_content_len;
            // Check if exactly one character was added and it's a newline
            if content_len > prev_len && clamped > 0 {
                let content = &app.editor.active_doc().content;
                if content.as_bytes().get(clamped - 1) == Some(&b'\n') {
                    // Get indentation of the previous line
                    let before_newline = &content[..clamped - 1];
                    let prev_line_start = before_newline.rfind('\n').map(|i| i + 1).unwrap_or(0);
                    let prev_line = &content[prev_line_start..clamped - 1];
                    let indent: String = prev_line
                        .chars()
                        .take_while(|c| *c == ' ' || *c == '\t')
                        .collect();

                    if !indent.is_empty() {
                        // Check if there's already indentation after the newline
                        let after = &content[clamped..];
                        let existing_indent: String = after
                            .chars()
                            .take_while(|c| *c == ' ' || *c == '\t')
                            .collect();
                        if existing_indent.is_empty() {
                            app.editor
                                .active_doc_mut()
                                .content
                                .insert_str(clamped, &indent);
                        }
                    }
                }
            }
            app.view.prev_content_len = app.editor.active_doc().content.len();
        }
    }
}

/// Highlight the current line with a subtle background.
fn highlight_current_line(
    ui: &egui::Ui,
    output: &egui::text_edit::TextEditOutput,
    theme: &rust_notepad::theme::AppTheme,
) {
    let cursor_range = match &output.cursor_range {
        Some(r) => r,
        None => return,
    };

    let galley = &output.galley;
    let text_rect = output.response.rect;
    let cursor = &cursor_range.primary;
    let pos = galley.pos_from_cursor(*cursor);
    let row_y = text_rect.top() + pos.min.y;
    let row_height = pos.max.y - pos.min.y;

    if row_height > 0.0 {
        let line_rect = egui::Rect::from_min_max(
            egui::pos2(text_rect.left(), row_y),
            egui::pos2(text_rect.right(), row_y + row_height),
        );
        ui.painter()
            .rect_filled(line_rect, 0.0, theme.current_line_bg());
    }
}

/// Highlight matching brace/bracket/parenthesis at cursor.
fn highlight_matching_brace(
    ui: &egui::Ui,
    output: &egui::text_edit::TextEditOutput,
    content: &str,
    theme: &rust_notepad::theme::AppTheme,
) {
    let cursor_range = match &output.cursor_range {
        Some(r) => r,
        None => return,
    };
    let cursor_pos = cursor_range.primary.index.min(content.len());

    let at_cursor = if cursor_pos < content.len() {
        Some(content.as_bytes()[cursor_pos])
    } else {
        None
    };
    let before_cursor = if cursor_pos > 0 {
        Some(content.as_bytes()[cursor_pos - 1])
    } else {
        None
    };

    let (brace_pos, is_open) = if matches!(at_cursor, Some(b'{') | Some(b'(') | Some(b'[')) {
        (cursor_pos, true)
    } else if matches!(before_cursor, Some(b'}') | Some(b')') | Some(b']')) {
        (cursor_pos - 1, false)
    } else if matches!(at_cursor, Some(b'}') | Some(b')') | Some(b']')) {
        (cursor_pos, false)
    } else if matches!(before_cursor, Some(b'{') | Some(b'(') | Some(b'[')) {
        (cursor_pos - 1, true)
    } else {
        return;
    };

    let brace = content.as_bytes()[brace_pos];
    let (open, close) = match brace {
        b'{' | b'}' => (b'{', b'}'),
        b'(' | b')' => (b'(', b')'),
        b'[' | b']' => (b'[', b']'),
        _ => return,
    };

    let match_pos = if is_open {
        find_matching_close_brace(content, brace_pos, open, close)
    } else {
        find_matching_open_brace(content, brace_pos, open, close)
    };

    if let Some(match_pos) = match_pos {
        let galley = &output.galley;
        let text_rect = output.response.rect;
        let clip = ui.clip_rect();
        let color = theme.brace_match_bg();

        for &pos in &[brace_pos, match_pos] {
            let p_start = galley.pos_from_cursor(egui::text::CCursor::new(pos));
            let p_end = galley.pos_from_cursor(egui::text::CCursor::new(pos + 1));

            let rect = egui::Rect::from_min_max(
                egui::pos2(
                    text_rect.left() + p_start.min.x,
                    text_rect.top() + p_start.min.y,
                ),
                egui::pos2(
                    text_rect.left() + p_end.max.x,
                    text_rect.top() + p_end.max.y,
                ),
            );

            if rect.top() < clip.bottom() + 50.0 && rect.bottom() > clip.top() - 50.0 {
                ui.painter().rect_filled(rect, 2.0, color);
            }
        }
    }
}

fn find_matching_close_brace(content: &str, start: usize, open: u8, close: u8) -> Option<usize> {
    let bytes = content.as_bytes();
    let mut depth = 1i32;
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == open {
            depth += 1;
        } else if bytes[i] == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn find_matching_open_brace(content: &str, start: usize, open: u8, close: u8) -> Option<usize> {
    let bytes = content.as_bytes();
    let mut depth = 1i32;
    let mut i = start;
    while i > 0 {
        i -= 1;
        if bytes[i] == close {
            depth += 1;
        } else if bytes[i] == open {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

struct GutterParams {
    font_size: f32,
    digit_count: usize,
    show_line_numbers: bool,
    fold_marker_width: f32,
}

/// Paint gutter (line numbers + fold markers) using actual galley row positions.
#[allow(clippy::too_many_arguments)]
fn paint_gutter(
    ui: &egui::Ui,
    app: &mut RustNotepadApp,
    editor_output: &egui::text_edit::TextEditOutput,
    line_map: &[usize],
    gutter_rect: egui::Rect,
    gutter_response: &egui::Response,
    params: &GutterParams,
) {
    let font_size = params.font_size;
    let digit_count = params.digit_count;
    let show_line_numbers = params.show_line_numbers;
    let fold_marker_width = params.fold_marker_width;
    let gutter_bg = app.app_theme.gutter_bg();
    let number_color = app.app_theme.gutter_text();
    let accent = app.app_theme.accent();
    let mono = FontId::monospace(font_size);
    let fold_font = FontId::monospace(font_size * 0.75);

    let galley = &editor_output.galley;
    let text_rect = editor_output.response.rect;

    // Extend gutter to match the editor height
    let full_gutter = egui::Rect::from_min_max(
        gutter_rect.min,
        egui::pos2(
            gutter_rect.right(),
            gutter_rect.bottom().max(text_rect.bottom()),
        ),
    );
    ui.painter().rect_filled(full_gutter, 0.0, gutter_bg);

    let clip = ui.clip_rect();
    let mut clicked_fold_line: Option<usize> = None;

    // Use galley's pos_from_cursor to get exact Y position for each display line.
    // Walk through display lines by finding the char offset of each line start.
    let galley_text = galley.text();
    let mut line_start_offsets: Vec<usize> = vec![0];
    for (i, ch) in galley_text.char_indices() {
        if ch == '\n' {
            line_start_offsets.push(i + 1);
        }
    }

    for (display_idx, &real_line) in line_map.iter().enumerate() {
        // Get the Y position of this display line from the galley
        let char_offset = line_start_offsets
            .get(display_idx)
            .copied()
            .unwrap_or(galley_text.len());
        let pos_rect = galley.pos_from_cursor(egui::text::CCursor::new(char_offset));
        let row_y = text_rect.top() + pos_rect.min.y;
        let row_height = pos_rect.max.y - pos_rect.min.y;

        // Skip if off-screen
        if row_y > clip.bottom() + 50.0 {
            break;
        }
        if row_y < clip.top() - 50.0 {
            continue;
        }

        // Line number
        if show_line_numbers {
            let num_str = format!("{:>width$}", real_line + 1, width = digit_count);
            let pos = egui::pos2(gutter_rect.left() + 4.0, row_y);
            ui.painter()
                .text(pos, Align2::LEFT_TOP, &num_str, mono.clone(), number_color);
        }

        // Fold marker
        let is_fold_start = app.editor.active_doc().fold_state.is_fold_start(real_line);
        if is_fold_start {
            let is_collapsed = app.editor.active_doc().fold_state.is_collapsed(real_line);
            let marker = if is_collapsed { "\u{25B6}" } else { "\u{25BC}" }; // ▶ / ▼
            let marker_x = gutter_rect.right() - fold_marker_width;
            let marker_pos = egui::pos2(marker_x, row_y);
            ui.painter().text(
                marker_pos,
                Align2::LEFT_TOP,
                marker,
                fold_font.clone(),
                accent,
            );

            // Check click on fold marker
            if gutter_response.clicked() {
                if let Some(click_pos) = gutter_response.interact_pointer_pos() {
                    if click_pos.y >= row_y
                        && click_pos.y <= row_y + row_height.max(font_size) + 2.0
                    {
                        clicked_fold_line = Some(real_line);
                    }
                }
            }
        }
    }

    // Apply fold toggle
    if let Some(real_line) = clicked_fold_line {
        app.editor.active_doc_mut().fold_state.toggle(real_line);
    }
}

/// Paint `·` for spaces and `→` for tabs over the editor text.
fn paint_whitespace_markers(
    ui: &egui::Ui,
    output: &egui::text_edit::TextEditOutput,
    content: &str,
    font_size: f32,
    color: egui::Color32,
) {
    let galley = &output.galley;
    let text_rect = output.response.rect;
    let mono = FontId::monospace(font_size * 0.7);
    let clip = ui.clip_rect();

    let mut char_idx = 0usize;
    for ch in content.chars() {
        let symbol = match ch {
            ' ' => "\u{00B7}",
            '\t' => "\u{2192}",
            _ => {
                char_idx += 1;
                continue;
            }
        };

        let ccursor = egui::text::CCursor::new(char_idx);
        let pos = galley.pos_from_cursor(ccursor);

        let screen_pos = egui::pos2(text_rect.left() + pos.min.x, text_rect.top() + pos.min.y);

        if screen_pos.y > clip.bottom() + 50.0 || screen_pos.y < clip.top() - 50.0 {
            char_idx += 1;
            continue;
        }

        ui.painter()
            .text(screen_pos, Align2::LEFT_TOP, symbol, mono.clone(), color);

        char_idx += 1;
    }
}

/// When a whole word is selected, highlight all other occurrences.
fn highlight_word_occurrences(
    ui: &egui::Ui,
    output: &egui::text_edit::TextEditOutput,
    content: &str,
) {
    let cursor_range = match &output.cursor_range {
        Some(r) => r,
        None => return,
    };

    let a = cursor_range.primary.index;
    let b = cursor_range.secondary.index;
    if a == b {
        return;
    }
    let start = a.min(b);
    let end = a.max(b);
    if end > content.len() || start >= end {
        return;
    }

    let selected = &content[start..end];
    if selected.is_empty() || selected.len() > 100 || selected.contains(char::is_whitespace) {
        return;
    }

    let before_ok = start == 0 || !content[..start].ends_with(|c: char| c.is_alphanumeric());
    let after_ok =
        end >= content.len() || !content[end..].starts_with(|c: char| c.is_alphanumeric());
    if !before_ok || !after_ok {
        return;
    }

    let galley = &output.galley;
    let text_rect = output.response.rect;
    let clip = ui.clip_rect();

    let highlight_color = if ui.visuals().dark_mode {
        egui::Color32::from_rgba_unmultiplied(255, 255, 0, 50)
    } else {
        egui::Color32::from_rgba_unmultiplied(255, 200, 0, 120)
    };

    let mut search_from = 0;
    while let Some(pos) = content[search_from..].find(selected) {
        let match_start = search_from + pos;
        let match_end = match_start + selected.len();

        if match_start == start {
            search_from = match_end;
            continue;
        }

        let b_ok =
            match_start == 0 || !content[..match_start].ends_with(|c: char| c.is_alphanumeric());
        let a_ok = match_end >= content.len()
            || !content[match_end..].starts_with(|c: char| c.is_alphanumeric());
        if !b_ok || !a_ok {
            search_from = match_end;
            continue;
        }

        let pos_start = galley.pos_from_cursor(egui::text::CCursor::new(match_start));
        let pos_end = galley.pos_from_cursor(egui::text::CCursor::new(match_end));

        let rect = egui::Rect::from_min_max(
            egui::pos2(
                text_rect.left() + pos_start.min.x,
                text_rect.top() + pos_start.min.y,
            ),
            egui::pos2(
                text_rect.left() + pos_end.max.x,
                text_rect.top() + pos_end.max.y,
            ),
        );

        if rect.top() < clip.bottom() + 50.0 && rect.bottom() > clip.top() - 50.0 {
            ui.painter().rect_filled(rect, 2.0, highlight_color);
        }

        search_from = match_end;
    }
}

/// For XML/HTML files, highlight matching open/close tag.
/// Find the tag at or near cursor_pos and highlight its matching partner.
fn highlight_matching_tag(ui: &egui::Ui, output: &egui::text_edit::TextEditOutput, content: &str) {
    let cursor_range = match &output.cursor_range {
        Some(r) => r,
        None => return,
    };
    let cursor_pos = cursor_range.primary.index.min(content.len());

    // Try to find a tag that the cursor is inside or touching.
    // Strategy: look backward for '<', check if cursor is within that tag's <...> span.
    // Also check if cursor is right after '>' (just clicked past a tag end).

    if let Some(tag) = find_tag_at_cursor(content, cursor_pos) {
        highlight_tag_pair(ui, output, content, tag.0, tag.1);
    }
}

/// Find the <tag> or </tag> at or near the cursor position.
/// Returns (tag_start, tag_end) byte offsets, or None.
fn find_tag_at_cursor(content: &str, cursor: usize) -> Option<(usize, usize)> {
    let bytes = content.as_bytes();

    // Case 1: cursor is inside a tag — look back for '<' that hasn't been closed by '>'
    if let Some(lt_pos) = content[..cursor].rfind('<') {
        // Check there's no '>' between lt_pos and cursor (cursor is inside the tag)
        if !content[lt_pos..cursor].contains('>') {
            if let Some(gt_offset) = content[lt_pos..].find('>') {
                return Some((lt_pos, lt_pos + gt_offset + 1));
            }
        }
    }

    // Case 2: cursor is right after '>' — the tag just ended
    if cursor > 0 && cursor <= bytes.len() && bytes[cursor - 1] == b'>' {
        if let Some(lt_pos) = content[..cursor].rfind('<') {
            return Some((lt_pos, cursor));
        }
    }

    // Case 3: cursor is right before '<' — about to enter a tag
    if cursor < bytes.len() && bytes[cursor] == b'<' {
        if let Some(gt_offset) = content[cursor..].find('>') {
            return Some((cursor, cursor + gt_offset + 1));
        }
    }

    // Case 4: cursor is right after '<' (e.g. clicked between < and tag name)
    if cursor > 0 && cursor <= bytes.len() && bytes[cursor - 1] == b'<' {
        let lt_pos = cursor - 1;
        if let Some(gt_offset) = content[lt_pos..].find('>') {
            return Some((lt_pos, lt_pos + gt_offset + 1));
        }
    }

    None
}

fn highlight_tag_pair(
    ui: &egui::Ui,
    output: &egui::text_edit::TextEditOutput,
    content: &str,
    tag_start: usize,
    tag_end: usize,
) {
    let tag_text = &content[tag_start..tag_end];
    let is_closing = tag_text.starts_with("</");
    let inner = if is_closing {
        &tag_text[2..tag_text.len() - 1]
    } else if tag_text.starts_with('<') {
        &tag_text[1..tag_text.len() - 1]
    } else {
        return;
    };

    if inner.ends_with('/') {
        return;
    }

    let tag_name = inner
        .split(|c: char| c.is_whitespace() || c == '/')
        .next()
        .unwrap_or("");
    if tag_name.is_empty() || tag_name.starts_with('!') || tag_name.starts_with('?') {
        return;
    }

    let match_range = if is_closing {
        find_matching_open_tag(content, tag_start, tag_name)
    } else {
        find_matching_close_tag(content, tag_end, tag_name)
    };

    let (match_start, match_end) = match match_range {
        Some(r) => r,
        None => return,
    };

    let galley = &output.galley;
    let text_rect = output.response.rect;
    let clip = ui.clip_rect();

    let tag_color = if ui.visuals().dark_mode {
        egui::Color32::from_rgba_unmultiplied(100, 200, 255, 50)
    } else {
        egui::Color32::from_rgba_unmultiplied(0, 100, 200, 140)
    };

    for &(s, e) in &[(tag_start, tag_end), (match_start, match_end)] {
        let p_start = galley.pos_from_cursor(egui::text::CCursor::new(s));
        let p_end = galley.pos_from_cursor(egui::text::CCursor::new(e));

        let rect = egui::Rect::from_min_max(
            egui::pos2(
                text_rect.left() + p_start.min.x,
                text_rect.top() + p_start.min.y,
            ),
            egui::pos2(
                text_rect.left() + p_end.max.x,
                text_rect.top() + p_end.max.y,
            ),
        );

        if rect.top() < clip.bottom() + 50.0 && rect.bottom() > clip.top() - 50.0 {
            ui.painter().rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(1.5, tag_color),
                egui::epaint::StrokeKind::Outside,
            );
        }
    }
}

fn find_matching_close_tag(content: &str, after: usize, tag_name: &str) -> Option<(usize, usize)> {
    let open_pattern = format!("<{}", tag_name);
    let close_pattern = format!("</{}", tag_name);
    let mut depth = 1i32;
    let mut pos = after;

    while pos < content.len() {
        let next_open = content[pos..].find(&open_pattern);
        let next_close = content[pos..].find(&close_pattern);

        let close_first = match (next_open, next_close) {
            (None, None) => break,
            (None, Some(_)) => true,
            (Some(_), None) => false,
            (Some(oi), Some(ci)) => ci <= oi,
        };

        if close_first {
            let ci = next_close.unwrap();
            let abs = pos + ci;
            let after_name = abs + close_pattern.len();
            if after_name < content.len() {
                let next_ch = content.as_bytes()[after_name];
                if next_ch == b'>' || next_ch == b' ' || next_ch == b'\t' {
                    depth -= 1;
                    if depth == 0 {
                        let end = content[abs..].find('>')? + abs + 1;
                        return Some((abs, end));
                    }
                }
            }
            pos = abs + close_pattern.len();
        } else {
            let oi = next_open.unwrap();
            let abs = pos + oi;
            let after_name = abs + open_pattern.len();
            if after_name < content.len() {
                let next_ch = content.as_bytes()[after_name];
                if next_ch == b'>' || next_ch == b' ' || next_ch == b'\t' {
                    if let Some(tag_close) = content[abs..].find('>') {
                        let tag_slice = &content[abs..abs + tag_close];
                        if !tag_slice.ends_with('/') {
                            depth += 1;
                        }
                    }
                }
            }
            pos = abs + open_pattern.len();
        }
    }
    None
}

fn find_matching_open_tag(content: &str, before: usize, tag_name: &str) -> Option<(usize, usize)> {
    let open_pattern = format!("<{}", tag_name);
    let close_pattern = format!("</{}", tag_name);
    let mut depth = 1i32;

    let search_area = &content[..before];
    let mut tags: Vec<(usize, bool)> = Vec::new();

    let mut pos = 0;
    while pos < search_area.len() {
        if let Some(ci) = search_area[pos..].find(&close_pattern) {
            let abs = pos + ci;
            let after_name = abs + close_pattern.len();
            if after_name < search_area.len() {
                let next_ch = search_area.as_bytes()[after_name];
                if next_ch == b'>' || next_ch == b' ' || next_ch == b'\t' {
                    tags.push((abs, true));
                }
            }
            if let Some(oi) = search_area[pos..].find(&open_pattern) {
                let abs_o = pos + oi;
                if abs_o < abs {
                    let after_name_o = abs_o + open_pattern.len();
                    if after_name_o < search_area.len() {
                        let next_ch = search_area.as_bytes()[after_name_o];
                        if next_ch == b'>' || next_ch == b' ' || next_ch == b'\t' || next_ch == b'/'
                        {
                            if let Some(tag_close) = search_area[abs_o..].find('>') {
                                let tag_slice = &search_area[abs_o..abs_o + tag_close];
                                if !tag_slice.ends_with('/') {
                                    tags.push((abs_o, false));
                                }
                            }
                        }
                    }
                    pos = abs_o + 1;
                    continue;
                }
            }
            pos = abs + 1;
        } else if let Some(oi) = search_area[pos..].find(&open_pattern) {
            let abs = pos + oi;
            let after_name = abs + open_pattern.len();
            if after_name < search_area.len() {
                let next_ch = search_area.as_bytes()[after_name];
                if next_ch == b'>' || next_ch == b' ' || next_ch == b'\t' || next_ch == b'/' {
                    if let Some(tag_close) = search_area[abs..].find('>') {
                        let tag_slice = &search_area[abs..abs + tag_close];
                        if !tag_slice.ends_with('/') {
                            tags.push((abs, false));
                        }
                    }
                }
            }
            pos = abs + 1;
        } else {
            break;
        }
    }

    for &(tag_pos, is_close) in tags.iter().rev() {
        if is_close {
            depth += 1;
        } else {
            depth -= 1;
            if depth == 0 {
                let end = content[tag_pos..].find('>')? + tag_pos + 1;
                return Some((tag_pos, end));
            }
        }
    }
    None
}
