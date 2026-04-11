use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::config::ThemePalette;
use crate::state::ssh::{SshAuthMethod, SshConnectionState, SshDialogField};
use crate::ui::styles::{modal_backdrop_style, overlay_footer_style, overlay_title_style};

/// Render the SSH connection dialog
pub fn render_ssh_connect_dialog(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &SshConnectionState,
    palette: &ThemePalette,
) {
    // Calculate dialog dimensions
    let width = 60.min(area.width.saturating_sub(4));
    let height = 14.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    // Render backdrop
    crate::ui::overlay::render_modal_backdrop(frame, area, dialog_area, *palette);

    // Create dialog block
    let block = Block::default()
        .title(Span::styled(" SSH Connect ", overlay_title_style(*palette)))
        .borders(Borders::ALL)
        .style(modal_backdrop_style(*palette));

    // Render the block
    frame.render_widget(block, dialog_area);

    // Inner area for content
    let inner = dialog_area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 2,
    });

    // Layout for fields
    let chunks = Layout::default()
        .constraints([
            Constraint::Length(1), // Address label
            Constraint::Length(3), // Address input
            Constraint::Length(1), // Auth method
            Constraint::Length(3), // Credential input
            Constraint::Length(1), // Error message
            Constraint::Length(1), // Footer
        ])
        .split(inner);

    // Address field
    let address_label = if state.focused_field == SshDialogField::Address {
        format!("Address: {}█", state.address)
    } else {
        format!("Address: {}", state.address)
    };
    let address_paragraph = Paragraph::new(address_label)
        .style(modal_backdrop_style(*palette));
    frame.render_widget(address_paragraph, chunks[0]);

    // Auth method
    let auth_text = match state.auth_method {
        SshAuthMethod::Password => {
            if state.focused_field == SshDialogField::Credential {
                "Auth: [Password] / Key File".to_string()
            } else {
                "Auth: Password / [Key File]".to_string()
            }
        }
        SshAuthMethod::KeyFile => {
            if state.focused_field == SshDialogField::Credential {
                "Auth: [Password] / Key File".to_string()
            } else {
                "Auth: Password / [Key File]".to_string()
            }
        }
    };
    let auth_paragraph = Paragraph::new(auth_text)
        .style(modal_backdrop_style(*palette));
    frame.render_widget(auth_paragraph, chunks[1]);

    // Credential field
    let credential_display = if state.auth_method == SshAuthMethod::Password {
        "•".repeat(state.credential.len())
    } else {
        state.credential.clone()
    };
    let credential_label = if state.focused_field == SshDialogField::Credential {
        format!("{}: {}█", 
            if state.auth_method == SshAuthMethod::Password { "Password" } else { "Key File" },
            credential_display
        )
    } else {
        format!("{}: {}", 
            if state.auth_method == SshAuthMethod::Password { "Password" } else { "Key File" },
            credential_display
        )
    };
    let credential_paragraph = Paragraph::new(credential_label)
        .style(modal_backdrop_style(*palette));
    frame.render_widget(credential_paragraph, chunks[2]);

    // Error message
    if let Some(error) = &state.error {
        let error_paragraph = Paragraph::new(format!("Error: {}", error))
            .style(Style::default().fg(ratatui::style::Color::Red));
        frame.render_widget(error_paragraph, chunks[3]);
    }

    // Footer
    let footer = Paragraph::new("Enter=connect Tab=switch Space=auth Esc=cancel")
        .style(overlay_footer_style(*palette));
    frame.render_widget(footer, chunks[4]);
}