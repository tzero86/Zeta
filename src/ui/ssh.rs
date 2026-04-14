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
    let address_paragraph = Paragraph::new(address_label).style(modal_backdrop_style(*palette));
    frame.render_widget(address_paragraph, chunks[0]);

    // Auth method
    let auth_text = match state.auth_method {
        SshAuthMethod::Password => "Auth: [Password] / Key File / Agent".to_string(),
        SshAuthMethod::KeyFile => "Auth: Password / [Key File] / Agent".to_string(),
        SshAuthMethod::Agent => "Auth: Password / Key File / [Agent]".to_string(),
    };
    let auth_paragraph = Paragraph::new(auth_text).style(modal_backdrop_style(*palette));
    frame.render_widget(auth_paragraph, chunks[1]);

    // Credential field
    let credential_display = if state.auth_method == SshAuthMethod::Password {
        "•".repeat(state.credential.len())
    } else {
        state.credential.clone()
    };

    let cred_name = match state.auth_method {
        SshAuthMethod::Password => "Password",
        SshAuthMethod::KeyFile => "Key File",
        SshAuthMethod::Agent => "Agent (not used)",
    };

    let credential_label = if state.focused_field == SshDialogField::Credential {
        format!("{}: {}█", cred_name, credential_display)
    } else {
        format!("{}: {}", cred_name, credential_display)
    };
    let credential_paragraph =
        Paragraph::new(credential_label).style(modal_backdrop_style(*palette));
    frame.render_widget(credential_paragraph, chunks[2]);

    // Error message
    if let Some(error) = &state.error {
        let display_error = if error.contains("failed:") {
            error.clone()
        } else {
            format!("Error: {}", error)
        };
        let error_paragraph =
            Paragraph::new(display_error).style(Style::default().fg(ratatui::style::Color::Red));
        frame.render_widget(error_paragraph, chunks[3]);
    }

    // Footer
    let footer = Paragraph::new("Enter=connect Tab=switch Space=auth Esc=cancel")
        .style(overlay_footer_style(*palette));
    frame.render_widget(footer, chunks[4]);
}

/// Render the SSH host-trust prompt.
///
/// Shows the host, port, and MD5 fingerprint so the user can verify the server's
/// identity before connecting. Enter/Y accepts; Esc/N rejects.
pub fn render_ssh_trust_prompt(
    frame: &mut Frame<'_>,
    area: Rect,
    host: &str,
    port: u16,
    fingerprint: &str,
    palette: &ThemePalette,
) {
    use ratatui::layout::Direction;
    use ratatui::widgets::Clear;

    let width = 64.min(area.width.saturating_sub(4));
    let height = 10.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    crate::ui::overlay::render_modal_backdrop(frame, area, dialog_area, *palette);
    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(Span::styled(
            " Unknown SSH Host ",
            overlay_title_style(*palette),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.prompt_border))
        .style(Style::default().bg(palette.surface_bg));
    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(1), // blank
            ratatui::layout::Constraint::Length(1), // host:port
            ratatui::layout::Constraint::Length(1), // fingerprint
            ratatui::layout::Constraint::Length(1), // blank
            ratatui::layout::Constraint::Min(1),    // body
            ratatui::layout::Constraint::Length(1), // footer
        ])
        .split(inner);

    let host_text = format!("  Host:        {}:{}", host, port);
    frame.render_widget(
        Paragraph::new(host_text).style(
            Style::default()
                .fg(palette.text_primary)
                .bg(palette.surface_bg),
        ),
        chunks[1],
    );
    let fp_text = format!("  Fingerprint: {}", fingerprint);
    frame.render_widget(
        Paragraph::new(fp_text).style(
            Style::default()
                .fg(palette.text_muted)
                .bg(palette.surface_bg),
        ),
        chunks[2],
    );

    let body = "  The host key is not in ~/.ssh/known_hosts.\n  Verify the fingerprint with the server administrator.";
    frame.render_widget(
        Paragraph::new(body).style(
            Style::default()
                .fg(palette.text_primary)
                .bg(palette.surface_bg),
        ),
        chunks[4],
    );

    let footer = Paragraph::new("Enter/Y=trust and connect  Esc/N=cancel")
        .style(overlay_footer_style(*palette));
    frame.render_widget(footer, chunks[5]);
}
