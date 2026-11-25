use gtk4::{self as gtk, gio, glib, prelude::*};
use std::{
  cell::{Cell, RefCell},
  collections::HashMap,
  io::Cursor,
  rc::Rc,
  time::Duration,
};

use crate::{
  app::{App, ComponentStatus, locales, old_data_warning, running_warning},
  constants::VERSION,
  df_binary::OS,
  localization::{LOCALE, t},
  ui_layout::{
    ContentColumn, ContentRow, ControlId, FooterItem, FooterLinkTarget, INSTALL_HEIGHT, INSTALL_WIDTH, INSTALLER_LAYOUT,
  },
};

struct Window {
  window: gtk::ApplicationWindow,
  app: RefCell<App>,
  startup_warning_shown: Cell<bool>,
  file_picker_active: Cell<bool>,
  game_path_label: gtk::Label,
  game_version_label: gtk::Label,
  game_checksum_label: gtk::Label,
  game_path_value: gtk::Label,
  game_version_value: gtk::Label,
  game_platform_label: gtk::Label,
  game_linux_icon: gtk::Image,
  game_windows_icon: gtk::Image,
  game_checksum_value: gtk::Label,
  browse_executable_button: gtk::Button,
  hook_section_heading: gtk::Label,
  hook_version_label: gtk::Label,
  hook_status_row: StatusRow,
  dictionary_section_heading: gtk::Label,
  dictionary_status_row: StatusRow,
  dictionary_language_selector: gtk::ComboBoxText,
  interface_language_selector: gtk::ComboBoxText,
  dictionary_language_options: RefCell<Vec<String>>,
  install_or_update_button: gtk::Button,
  remove_localization_button: gtk::Button,
  operation_spinner: gtk::Spinner,
  operation_notice_label: gtk::Label,
  bug_report_link_label: gtk::Label,
  translation_help_link_label: gtk::Label,
}

pub fn run() -> anyhow::Result<()> {
  gtk::init().map_err(|err| anyhow::anyhow!("{err}"))?;
  let application = gtk::Application::new(Some("io.github.dfint.Installer"), gio::ApplicationFlags::NON_UNIQUE);
  application.connect_activate(|application| {
    let ui = Window::new(application);
    ui.app.borrow_mut().on_start();
    ui.refresh();
    ui.window.present();
    glib::timeout_add_local(Duration::from_millis(100), move || {
      if !ui.window.is_visible() {
        return glib::ControlFlow::Break;
      }
      let changed = ui.app.borrow_mut().update_state();
      if changed {
        ui.changed();
      }
      glib::ControlFlow::Continue
    });
  });
  application.run_with_args::<&str>(&[]);
  Ok(())
}

impl Window {
  fn new(application: &gtk::Application) -> Rc<Self> {
    let window = gtk::ApplicationWindow::builder()
      .application(application)
      .title("DF localization installer")
      .default_width(INSTALLER_LAYOUT.width)
      .default_height(INSTALLER_LAYOUT.height)
      .build();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    window.set_child(Some(&root));

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.set_margin_top(12);
    content.set_margin_bottom(8);
    content.set_margin_start(10);
    content.set_margin_end(10);
    content.set_vexpand(true);
    root.append(&content);

    let grid = gtk::Grid::builder()
      .row_spacing(6)
      .column_spacing(12)
      .build();
    grid.set_vexpand(true);
    content.append(&grid);
    let game_details_heading = heading("Dwarf Fortress");
    let game_path_label = label();
    let game_version_label = label();
    let game_checksum_label = label();
    let game_path_value = label();
    game_path_value.set_selectable(true);
    game_path_value.set_single_line_mode(true);
    game_path_value.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    let game_version_value = label();
    let game_platform_label = label();
    let game_linux_icon = small_icon(include_bytes!("../assets/os-linux.png"));
    let game_windows_icon = small_icon(include_bytes!("../assets/os-windows.png"));
    let game_version_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    game_version_box.append(&game_version_value);
    game_version_box.append(&game_linux_icon);
    game_version_box.append(&game_windows_icon);
    game_version_box.append(&game_platform_label);
    let game_checksum_value = label();
    let browse_executable_button = gtk::Button::from_icon_name("system-search-symbolic");
    browse_executable_button.set_halign(gtk::Align::End);
    let hook_section_heading = heading("");
    let hook_version_label = label();
    let hook_status_row = StatusRow::new();

    let dictionary_section_heading = heading("");
    let dictionary_language_selector = gtk::ComboBoxText::new();
    dictionary_language_selector.set_size_request(140, -1);
    dictionary_language_selector.set_halign(gtk::Align::Start);
    let dictionary_status_row = StatusRow::new();

    let install_action_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    install_action_box.set_halign(gtk::Align::Center);
    install_action_box.set_margin_top(14);
    let install_or_update_button = gtk::Button::new();
    install_or_update_button.set_size_request(INSTALL_WIDTH, INSTALL_HEIGHT);
    install_or_update_button.add_css_class("suggested-action");
    let operation_spinner = gtk::Spinner::new();
    operation_spinner.set_size_request(24, INSTALL_HEIGHT);
    install_action_box.append(&install_or_update_button);
    install_action_box.append(&operation_spinner);

    let operation_notice_label = label();
    operation_notice_label.set_wrap(true);
    operation_notice_label.set_halign(gtk::Align::Start);
    operation_notice_label.set_valign(gtk::Align::End);
    operation_notice_label.set_vexpand(true);
    operation_notice_label.set_margin_top(6);
    let widgets: HashMap<_, &gtk::Widget> = HashMap::from([
      (ControlId::GameDetailsHeading, game_details_heading.upcast_ref()),
      (ControlId::GamePathLabel, game_path_label.upcast_ref()),
      (ControlId::GamePathValue, game_path_value.upcast_ref()),
      (ControlId::BrowseExecutable, browse_executable_button.upcast_ref()),
      (ControlId::GameVersionLabel, game_version_label.upcast_ref()),
      (ControlId::GameVersionValue, game_version_box.upcast_ref()),
      (ControlId::GameChecksumLabel, game_checksum_label.upcast_ref()),
      (ControlId::GameChecksumValue, game_checksum_value.upcast_ref()),
      (ControlId::HookHeading, hook_section_heading.upcast_ref()),
      (ControlId::HookVersionLabel, hook_version_label.upcast_ref()),
      (
        ControlId::HookInstalledVersion,
        hook_status_row.installed_version_label.upcast_ref(),
      ),
      (
        ControlId::HookAvailableVersion,
        hook_status_row.available_version_label.upcast_ref(),
      ),
      (
        ControlId::HookUpdateStatus,
        hook_status_row.update_status_label.upcast_ref(),
      ),
      (ControlId::DictionaryHeading, dictionary_section_heading.upcast_ref()),
      (
        ControlId::DictionaryLanguageSelector,
        dictionary_language_selector.upcast_ref(),
      ),
      (
        ControlId::DictionaryInstalledVersion,
        dictionary_status_row.installed_version_label.upcast_ref(),
      ),
      (
        ControlId::DictionaryAvailableVersion,
        dictionary_status_row.available_version_label.upcast_ref(),
      ),
      (
        ControlId::DictionaryUpdateStatus,
        dictionary_status_row.update_status_label.upcast_ref(),
      ),
      (ControlId::InstallOrUpdate, install_action_box.upcast_ref()),
      (ControlId::OperationNotice, operation_notice_label.upcast_ref()),
    ]);
    let mut row = 0;
    for item in INSTALLER_LAYOUT.content_rows() {
      let is_heading = matches!(item, ContentRow::Heading(_));
      for cell in item.cells() {
        let widget = widgets[&cell.control];
        if is_heading && row > 0 {
          widget.set_margin_top(12);
        }
        grid.attach(widget, cell.column.index(), row, cell.column_span, 1);
      }
      row += 1;
      if is_heading {
        grid.attach(
          &gtk::Separator::new(gtk::Orientation::Horizontal),
          0,
          row,
          ContentColumn::COUNT,
          1,
        );
        row += 1;
      }
    }

    root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    footer.set_margin_start(8);
    footer.set_margin_end(8);
    footer.set_margin_top(4);
    footer.set_margin_bottom(4);
    let (bug_report_link, bug_report_link_label) = footer_link(
      FooterLinkTarget::ReportBug.url(),
      include_bytes!("../assets/github.png"),
    );
    let (translation_help_link, translation_help_link_label) = footer_link(
      FooterLinkTarget::HelpWithTranslation.url(),
      include_bytes!("../assets/transifex.png"),
    );
    let interface_language_selector = gtk::ComboBoxText::new();
    for item in locales(&LOCALE.read().current_locale()) {
      interface_language_selector.append(Some(&item), &item);
    }
    let installer_version_label = gtk::Label::new(Some(&format!("v{VERSION}")));
    installer_version_label.add_css_class("dim-label");
    installer_version_label.set_xalign(0.0);
    let remove_localization_button = gtk::Button::from_icon_name("user-trash-symbolic");
    let footer_widgets: HashMap<_, &gtk::Widget> = HashMap::from([
      (
        FooterItem::ExternalLink(FooterLinkTarget::ReportBug),
        bug_report_link.upcast_ref(),
      ),
      (
        FooterItem::ExternalLink(FooterLinkTarget::HelpWithTranslation),
        translation_help_link.upcast_ref(),
      ),
      (FooterItem::InstallerVersion, installer_version_label.upcast_ref()),
      (
        FooterItem::InterfaceLanguageSelector,
        interface_language_selector.upcast_ref(),
      ),
      (
        FooterItem::RemoveLocalizationButton,
        remove_localization_button.upcast_ref(),
      ),
    ]);
    let append = |item| footer.append(footer_widgets[&item]);
    INSTALLER_LAYOUT
      .footer
      .leading_items
      .iter()
      .copied()
      .for_each(append);
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    footer.append(&spacer);
    INSTALLER_LAYOUT
      .footer
      .trailing_items
      .iter()
      .copied()
      .for_each(append);
    root.append(&footer);

    let ui = Rc::new(Self {
      window,
      app: RefCell::new(App::default()),
      game_path_label,
      game_version_label,
      game_checksum_label,
      game_path_value,
      game_version_value,
      game_platform_label,
      game_linux_icon,
      game_windows_icon,
      game_checksum_value,
      startup_warning_shown: Cell::default(),
      file_picker_active: Cell::default(),
      browse_executable_button,
      hook_section_heading,
      hook_version_label,
      hook_status_row,
      dictionary_section_heading,
      dictionary_status_row,
      dictionary_language_selector,
      interface_language_selector,
      dictionary_language_options: RefCell::default(),
      install_or_update_button,
      remove_localization_button,
      operation_spinner,
      operation_notice_label,
      bug_report_link_label,
      translation_help_link_label,
    });
    ui.connect();
    ui
  }

  fn connect(self: &Rc<Self>) {
    let weak = Rc::downgrade(self);
    self.browse_executable_button.connect_clicked(move |_| {
      if let Some(ui) = weak.upgrade() {
        ui.pick_file();
      }
    });
    let weak = Rc::downgrade(self);
    self.install_or_update_button.connect_clicked(move |_| {
      if let Some(ui) = weak.upgrade() {
        ui.app.borrow_mut().update_data();
        ui.refresh();
      }
    });
    let weak = Rc::downgrade(self);
    self.remove_localization_button.connect_clicked(move |_| {
      if let Some(ui) = weak.upgrade() {
        ui.confirm(&t!("Delete all localization files?"), App::confirm_localization_removal);
      }
    });
    let weak = Rc::downgrade(self);
    self
      .dictionary_language_selector
      .connect_changed(move |combo| {
        if let Some(ui) = weak.upgrade()
          && let Some(id) = combo.active_id()
        {
          {
            let Ok(mut app) = ui.app.try_borrow_mut() else { return };
            app.select_language(id.to_string());
          }
          ui.refresh();
        }
      });
    let weak = Rc::downgrade(self);
    self
      .interface_language_selector
      .connect_changed(move |combo| {
        if let Some(ui) = weak.upgrade()
          && let Some(id) = combo.active_id()
        {
          {
            let Ok(mut app) = ui.app.try_borrow_mut() else { return };
            app.select_locale(id.to_string());
          }
          ui.refresh();
        }
      });
    let weak = Rc::downgrade(self);
    self.window.connect_close_request(move |_| {
      if let Some(ui) = weak.upgrade() {
        ui.app.borrow_mut().on_close();
      }
      glib::Propagation::Proceed
    });
  }

  fn refresh(&self) {
    let app = self.app.borrow();
    let view = app.view();
    self.game_path_label.set_text(&t!("Path"));
    self.game_version_label.set_text(&t!("Version"));
    self.game_checksum_label.set_text(&t!("Checksum"));
    self.game_path_value.set_text(&view.path);
    self.game_path_value.set_tooltip_text(Some(&view.path));
    self.game_version_value.set_text(&view.version);
    self
      .game_linux_icon
      .set_visible(app.bin.valid && view.platform == OS::Linux);
    self
      .game_windows_icon
      .set_visible(app.bin.valid && view.platform == OS::Windows);
    self
      .game_platform_label
      .set_text(&format!("{} | {}", view.platform, view.distribution));
    self.game_checksum_value.set_text(&view.checksum);
    label_icon_button(&self.browse_executable_button, &t!("Open Dwarf Fortress executable"));
    self
      .browse_executable_button
      .set_sensitive(view.ready && !view.busy);
    set_heading(&self.hook_section_heading, &t!("Hook"));
    self.hook_version_label.set_text(&t!("Version"));
    self.hook_status_row.refresh(&view.hook);
    set_heading(&self.dictionary_section_heading, &t!("Dictionary"));
    self.dictionary_status_row.refresh(&view.dictionary);
    let mut options = self.dictionary_language_options.borrow_mut();
    if *options != view.languages {
      self.dictionary_language_selector.remove_all();
      for language in &view.languages {
        self
          .dictionary_language_selector
          .append(Some(language), language);
      }
      *options = view.languages;
    }
    self
      .dictionary_language_selector
      .set_active_id(Some(&app.selected_language));
    self
      .dictionary_language_selector
      .set_sensitive(view.ready && !view.busy && !options.is_empty());
    self
      .interface_language_selector
      .set_active_id(Some(&app.ui_locale));
    self.install_or_update_button.set_label(&view.action);
    self.install_or_update_button.set_visible(view.can_update);
    label_icon_button(&self.remove_localization_button, &t!("Delete localization files"));
    self
      .remove_localization_button
      .set_visible(view.has_localization);
    self
      .remove_localization_button
      .set_sensitive(view.ready && !view.busy);
    let spinning = !app.ready() || view.busy;
    self.operation_spinner.set_visible(spinning);
    self.operation_spinner.set_spinning(spinning);
    self
      .operation_notice_label
      .set_text(app.notice.as_deref().unwrap_or_default());
    self
      .bug_report_link_label
      .set_text(&FooterLinkTarget::ReportBug.title());
    self
      .translation_help_link_label
      .set_text(&FooterLinkTarget::HelpWithTranslation.title());
  }

  fn changed(self: &Rc<Self>) {
    self.refresh();
    let mut app = self.app.borrow_mut();
    if app.df_running {
      drop(app);
      if self.startup_warning_shown.replace(true) {
        return;
      }
      let dialog = self.warning(&running_warning(), &[(&t!("Ok"), gtk::ResponseType::Ok)]);
      let window = self.window.clone();
      dialog.connect_response(move |dialog, _| {
        dialog.destroy();
        window.close();
      });
      dialog.present();
    } else if app.ready() && !app.bin.valid {
      drop(app);
      self.pick_file();
    } else if app.ready() && app.delete_old_data_show {
      app.delete_old_data_show = false;
      drop(app);
      self.confirm(&old_data_warning(), App::confirm_old_data_removal);
    }
  }

  fn pick_file(self: &Rc<Self>) {
    if self.file_picker_active.replace(true) {
      return;
    }
    let dialog = gtk::FileChooserDialog::builder()
      .title(t!("Open Dwarf Fortress executable"))
      .transient_for(&self.window)
      .destroy_with_parent(true)
      .modal(true)
      .action(gtk::FileChooserAction::Open)
      .build();
    dialog.add_button(&t!("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&t!("Open"), gtk::ResponseType::Accept);
    dialog.set_default_response(gtk::ResponseType::Accept);
    let dir = self.app.borrow().bin.dir.clone();
    if dir.is_dir() {
      let _ = dialog.set_current_folder(Some(&gio::File::for_path(dir)));
    }
    let weak = Rc::downgrade(self);
    dialog.connect_response(move |dialog, response| {
      let path = dialog.file().and_then(|file| file.path());
      dialog.destroy();
      if let Some(ui) = weak.upgrade() {
        ui.file_picker_active.set(false);
        if response == gtk::ResponseType::Accept
          && let Some(path) = path
        {
          ui.app.borrow_mut().select_binary(path);
          if !ui.app.borrow().bin.valid {
            ui.refresh();
            let message = ui.app.borrow().notice.clone().unwrap_or_default();
            let warning = ui.warning(&message, &[(&t!("Ok"), gtk::ResponseType::Ok)]);
            ui.file_picker_active.set(true);
            let weak = Rc::downgrade(&ui);
            warning.connect_response(move |warning, _| {
              warning.destroy();
              if let Some(ui) = weak.upgrade() {
                ui.file_picker_active.set(false);
                ui.pick_file();
              }
            });
            warning.present();
          } else {
            ui.changed();
          }
        } else if !ui.app.borrow().bin.valid {
          ui.window.close();
        }
      }
    });
    dialog.present();
  }

  fn warning(&self, text: &str, buttons: &[(&str, gtk::ResponseType)]) -> gtk::MessageDialog {
    let dialog = gtk::MessageDialog::builder()
      .transient_for(&self.window)
      .destroy_with_parent(true)
      .modal(true)
      .message_type(gtk::MessageType::Warning)
      .text(t!("Warning"))
      .secondary_text(text)
      .build();
    dialog.add_buttons(buttons);
    dialog
  }

  fn confirm(self: &Rc<Self>, text: &str, accept: fn(&mut App)) {
    let dialog = self.warning(
      text,
      &[(&t!("No"), gtk::ResponseType::No), (&t!("Yes"), gtk::ResponseType::Yes)],
    );
    dialog.set_default_response(gtk::ResponseType::No);
    let weak = Rc::downgrade(self);
    dialog.connect_response(move |dialog, response| {
      dialog.destroy();
      if response == gtk::ResponseType::Yes
        && let Some(ui) = weak.upgrade()
      {
        accept(&mut ui.app.borrow_mut());
        ui.refresh();
      }
    });
    dialog.present();
  }
}

fn label() -> gtk::Label {
  gtk::Label::builder().xalign(0.0).build()
}

fn heading(text: &str) -> gtk::Label {
  let heading = label();
  set_heading(&heading, text);
  heading
}

fn set_heading(label: &gtk::Label, text: &str) {
  label.set_markup(&format!("<big>{}</big>", glib::markup_escape_text(text)));
}

struct StatusRow {
  installed_version_label: gtk::Label,
  available_version_label: gtk::Label,
  update_status_label: gtk::Label,
}

impl StatusRow {
  fn new() -> Self {
    let installed_version_label = label();
    let available_version_label = label();
    installed_version_label.set_size_request(135, -1);
    available_version_label.set_size_request(135, -1);
    let update_status_label = label();
    update_status_label.set_wrap(true);
    update_status_label.set_hexpand(true);
    Self {
      installed_version_label,
      available_version_label,
      update_status_label,
    }
  }

  fn refresh(&self, value: &ComponentStatus) {
    self.installed_version_label.set_text(&value.local);
    self.available_version_label.set_text(&value.available);
    let foreground = self.update_status_label.style_context().color();
    let (r, g, b) = value.status.color(foreground.green() > 0.5);
    self.update_status_label.set_markup(&format!(
      r##"<span foreground="#{r:02x}{g:02x}{b:02x}">{} {}</span>"##,
      value.status.symbol(),
      glib::markup_escape_text(&value.text),
    ));
  }
}

fn label_icon_button(button: &gtk::Button, title: &str) {
  button.set_tooltip_text(Some(title));
  button.update_property(&[gtk::accessible::Property::Label(title)]);
}

fn footer_link(uri: &str, icon: &'static [u8]) -> (gtk::LinkButton, gtk::Label) {
  let link = gtk::LinkButton::new(uri);
  let contents = gtk::Box::new(gtk::Orientation::Horizontal, 5);
  contents.append(&small_icon(icon));
  let title = label();
  contents.append(&title);
  link.set_child(Some(&contents));
  (link, title)
}

fn small_icon(bytes: &'static [u8]) -> gtk::Image {
  let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_read(Cursor::new(bytes))
    .ok()
    .and_then(|icon| icon.scale_simple(16, 16, gtk::gdk_pixbuf::InterpType::Bilinear));
  gtk::Image::from_pixbuf(pixbuf.as_ref())
}
