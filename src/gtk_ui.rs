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
  picking: Cell<bool>,
  path_title: gtk::Label,
  version_title: gtk::Label,
  checksum_title: gtk::Label,
  path: gtk::Label,
  version: gtk::Label,
  platform: gtk::Label,
  linux_icon: gtk::Image,
  windows_icon: gtk::Image,
  checksum: gtk::Label,
  browse: gtk::Button,
  hook_title: gtk::Label,
  hook_version: gtk::Label,
  hook: StatusRow,
  dict_title: gtk::Label,
  dict: StatusRow,
  languages: gtk::ComboBoxText,
  locale: gtk::ComboBoxText,
  language_options: RefCell<Vec<String>>,
  install: gtk::Button,
  remove: gtk::Button,
  spinner: gtk::Spinner,
  notice: gtk::Label,
  bugs: gtk::Label,
  translations: gtk::Label,
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
    let game_title = heading("Dwarf Fortress");
    let path_title = label();
    let version_title = label();
    let checksum_title = label();
    let path = label();
    path.set_selectable(true);
    path.set_single_line_mode(true);
    path.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    let version = label();
    let platform = label();
    let linux_icon = small_icon(include_bytes!("../assets/os-linux.png"));
    let windows_icon = small_icon(include_bytes!("../assets/os-windows.png"));
    let version_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    version_row.append(&version);
    version_row.append(&linux_icon);
    version_row.append(&windows_icon);
    version_row.append(&platform);
    let checksum = label();
    let browse = gtk::Button::from_icon_name("system-search-symbolic");
    browse.set_halign(gtk::Align::End);
    let hook_title = heading("");
    let hook_version = label();
    let hook = StatusRow::new();

    let dict_title = heading("");
    let languages = gtk::ComboBoxText::new();
    languages.set_size_request(140, -1);
    languages.set_halign(gtk::Align::Start);
    let dict = StatusRow::new();

    let actions = gtk::Box::new(gtk::Orientation::Vertical, 0);
    actions.set_halign(gtk::Align::Center);
    actions.set_margin_top(14);
    let install = gtk::Button::new();
    install.set_size_request(INSTALL_WIDTH, INSTALL_HEIGHT);
    install.add_css_class("suggested-action");
    let spinner = gtk::Spinner::new();
    spinner.set_size_request(24, INSTALL_HEIGHT);
    actions.append(&install);
    actions.append(&spinner);

    let notice = label();
    notice.set_wrap(true);
    notice.set_halign(gtk::Align::Start);
    notice.set_valign(gtk::Align::End);
    notice.set_vexpand(true);
    notice.set_margin_top(6);
    let widgets: HashMap<_, &gtk::Widget> = HashMap::from([
      (ControlId::GameDetailsHeading, game_title.upcast_ref()),
      (ControlId::GamePathLabel, path_title.upcast_ref()),
      (ControlId::GamePathValue, path.upcast_ref()),
      (ControlId::BrowseExecutable, browse.upcast_ref()),
      (ControlId::GameVersionLabel, version_title.upcast_ref()),
      (ControlId::GameVersionValue, version_row.upcast_ref()),
      (ControlId::GameChecksumLabel, checksum_title.upcast_ref()),
      (ControlId::GameChecksumValue, checksum.upcast_ref()),
      (ControlId::HookHeading, hook_title.upcast_ref()),
      (ControlId::HookVersionLabel, hook_version.upcast_ref()),
      (ControlId::HookInstalledVersion, hook.local.upcast_ref()),
      (ControlId::HookAvailableVersion, hook.available.upcast_ref()),
      (ControlId::HookUpdateStatus, hook.status.upcast_ref()),
      (ControlId::DictionaryHeading, dict_title.upcast_ref()),
      (ControlId::DictionaryLanguageSelector, languages.upcast_ref()),
      (ControlId::DictionaryInstalledVersion, dict.local.upcast_ref()),
      (ControlId::DictionaryAvailableVersion, dict.available.upcast_ref()),
      (ControlId::DictionaryUpdateStatus, dict.status.upcast_ref()),
      (ControlId::InstallOrUpdate, actions.upcast_ref()),
      (ControlId::OperationNotice, notice.upcast_ref()),
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
    let (bug_link, bugs) = footer_link(
      FooterLinkTarget::ReportBug.url(),
      include_bytes!("../assets/github.png"),
    );
    let (translation_link, translations) = footer_link(
      FooterLinkTarget::HelpWithTranslation.url(),
      include_bytes!("../assets/transifex.png"),
    );
    let locale = gtk::ComboBoxText::new();
    for item in locales(&LOCALE.read().current_locale()) {
      locale.append(Some(&item), &item);
    }
    let build = gtk::Label::new(Some(&format!("v{VERSION}")));
    build.add_css_class("dim-label");
    build.set_xalign(0.0);
    let remove = gtk::Button::from_icon_name("user-trash-symbolic");
    let footer_widgets: HashMap<_, &gtk::Widget> = HashMap::from([
      (
        FooterItem::ExternalLink(FooterLinkTarget::ReportBug),
        bug_link.upcast_ref(),
      ),
      (
        FooterItem::ExternalLink(FooterLinkTarget::HelpWithTranslation),
        translation_link.upcast_ref(),
      ),
      (FooterItem::InstallerVersion, build.upcast_ref()),
      (FooterItem::InterfaceLanguageSelector, locale.upcast_ref()),
      (FooterItem::RemoveLocalizationButton, remove.upcast_ref()),
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
      path_title,
      version_title,
      checksum_title,
      path,
      version,
      platform,
      linux_icon,
      windows_icon,
      checksum,
      startup_warning_shown: Cell::default(),
      picking: Cell::default(),
      browse,
      hook_title,
      hook_version,
      hook,
      dict_title,
      dict,
      languages,
      locale,
      language_options: RefCell::default(),
      install,
      remove,
      spinner,
      notice,
      bugs,
      translations,
    });
    ui.connect();
    ui
  }

  fn connect(self: &Rc<Self>) {
    let weak = Rc::downgrade(self);
    self.browse.connect_clicked(move |_| {
      if let Some(ui) = weak.upgrade() {
        ui.pick_file();
      }
    });
    let weak = Rc::downgrade(self);
    self.install.connect_clicked(move |_| {
      if let Some(ui) = weak.upgrade() {
        ui.app.borrow_mut().update_data();
        ui.refresh();
      }
    });
    let weak = Rc::downgrade(self);
    self.remove.connect_clicked(move |_| {
      if let Some(ui) = weak.upgrade() {
        ui.confirm(&t!("Delete all localization files?"), App::confirm_localization_removal);
      }
    });
    let weak = Rc::downgrade(self);
    self.languages.connect_changed(move |combo| {
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
    self.locale.connect_changed(move |combo| {
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
    self.path_title.set_text(&t!("Path"));
    self.version_title.set_text(&t!("Version"));
    self.checksum_title.set_text(&t!("Checksum"));
    self.path.set_text(&view.path);
    self.path.set_tooltip_text(Some(&view.path));
    self.version.set_text(&view.version);
    self
      .linux_icon
      .set_visible(app.bin.valid && view.platform == OS::Linux);
    self
      .windows_icon
      .set_visible(app.bin.valid && view.platform == OS::Windows);
    self
      .platform
      .set_text(&format!("{} | {}", view.platform, view.distribution));
    self.checksum.set_text(&view.checksum);
    label_icon_button(&self.browse, &t!("Open Dwarf Fortress executable"));
    self.browse.set_sensitive(view.ready && !view.busy);
    set_heading(&self.hook_title, &t!("Hook"));
    self.hook_version.set_text(&t!("Version"));
    self.hook.refresh(&view.hook);
    set_heading(&self.dict_title, &t!("Dictionary"));
    self.dict.refresh(&view.dictionary);
    let mut options = self.language_options.borrow_mut();
    if *options != view.languages {
      self.languages.remove_all();
      for language in &view.languages {
        self.languages.append(Some(language), language);
      }
      *options = view.languages;
    }
    self.languages.set_active_id(Some(&app.selected_language));
    self
      .languages
      .set_sensitive(view.ready && !view.busy && !options.is_empty());
    self.locale.set_active_id(Some(&app.ui_locale));
    self.install.set_label(&view.action);
    self.install.set_visible(view.can_update);
    label_icon_button(&self.remove, &t!("Delete localization files"));
    self.remove.set_visible(view.has_localization);
    self.remove.set_sensitive(view.ready && !view.busy);
    let spinning = !app.ready() || view.busy;
    self.spinner.set_visible(spinning);
    self.spinner.set_spinning(spinning);
    self
      .notice
      .set_text(app.notice.as_deref().unwrap_or_default());
    self.bugs.set_text(&FooterLinkTarget::ReportBug.title());
    self
      .translations
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
    if self.picking.replace(true) {
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
        ui.picking.set(false);
        if response == gtk::ResponseType::Accept
          && let Some(path) = path
        {
          ui.app.borrow_mut().select_binary(path);
          if !ui.app.borrow().bin.valid {
            ui.refresh();
            let message = ui.app.borrow().notice.clone().unwrap_or_default();
            let warning = ui.warning(&message, &[(&t!("Ok"), gtk::ResponseType::Ok)]);
            ui.picking.set(true);
            let weak = Rc::downgrade(&ui);
            warning.connect_response(move |warning, _| {
              warning.destroy();
              if let Some(ui) = weak.upgrade() {
                ui.picking.set(false);
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
  local: gtk::Label,
  available: gtk::Label,
  status: gtk::Label,
}

impl StatusRow {
  fn new() -> Self {
    let local = label();
    let available = label();
    local.set_size_request(135, -1);
    available.set_size_request(135, -1);
    let status = label();
    status.set_wrap(true);
    status.set_hexpand(true);
    Self {
      local,
      available,
      status,
    }
  }

  fn refresh(&self, value: &ComponentStatus) {
    self.local.set_text(&value.local);
    self.available.set_text(&value.available);
    let foreground = self.status.style_context().color();
    let (r, g, b) = value.status.color(foreground.green() > 0.5);
    self.status.set_markup(&format!(
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
