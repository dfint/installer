use std::{
  ffi::OsString,
  mem::size_of,
  os::windows::ffi::{OsStrExt, OsStringExt},
  path::{Path, PathBuf},
  ptr::{null, null_mut},
};

use windows_sys::Win32::{
  Foundation::{HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM},
  Graphics::Gdi::{
    COLOR_BTNFACE, CreateFontIndirectW, DeleteObject, GetDC, GetSysColor, GetSysColorBrush, GetTextExtentPoint32W, HDC,
    HFONT, InvalidateRect, ReleaseDC, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
  },
  System::LibraryLoader::GetModuleHandleW,
  UI::{
    Controls::{
      Dialogs::*, ICC_BAR_CLASSES, ICC_LINK_CLASS, INITCOMMONCONTROLSEX, InitCommonControlsEx, LM_GETIDEALSIZE,
      NM_CLICK, NM_RETURN, NMHDR, SB_SETMINHEIGHT, STATUSCLASSNAMEW, WC_LINK,
    },
    HiDpi::*,
    Input::KeyboardAndMouse::EnableWindow,
    Shell::{DefSubclassProc, SetWindowSubclass, ShellExecuteW},
    WindowsAndMessaging::*,
  },
};

use crate::{
  app::{App, ComponentStatus, Status, locales, old_data_warning, running_warning},
  constants::VERSION,
  df_binary::OS,
  localization::{LOCALE, t},
  ui_layout::{
    ContentColumn, ContentRow, ControlId, FooterItem, FooterLinkTarget, INSTALL_HEIGHT, INSTALL_WIDTH,
    INSTALLER_LAYOUT, SectionHeading, SectionRow,
  },
};

const SS_ICON: u32 = 0x0003;
const SS_NOTIFY: u32 = 0x0100;
const SS_ETCHEDHORZ: u32 = 0x0010;
const SS_NOPREFIX: u32 = 0x0080;
const ACTION: u32 = WM_APP + 1;
const BROWSE: usize = 100;
const LANGUAGE: usize = 101;
const LOCALE_SELECT: usize = 102;
const INSTALL: usize = 103;
const REMOVE: usize = 104;
const BUG_LINK: usize = 105;
const TRANSIFEX_LINK: usize = 106;
const BUG_ICON: usize = 107;
const TRANSIFEX_ICON: usize = 108;

const GITHUB_RESOURCE: usize = 2;
const TRANSIFEX_RESOURCE: usize = 3;
const TRASH_RESOURCE: usize = 4;
const LINUX_RESOURCE: usize = 9;
const WINDOWS_RESOURCE: usize = 10;

pub fn run() -> anyhow::Result<()> {
  // All HWNDs and the message loop are owned by this thread. The window procedure
  // posts control actions so a native modal dialog cannot reenter application state.
  unsafe {
    let common_controls = INITCOMMONCONTROLSEX {
      dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
      dwICC: ICC_BAR_CLASSES | ICC_LINK_CLASS,
    };
    anyhow::ensure!(
      InitCommonControlsEx(&common_controls) != 0,
      "Unable to initialize Windows common controls"
    );
    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
    let dpi = GetDpiForSystem().max(96);
    let instance = GetModuleHandleW(null());
    let class = wide("DfintInstaller");
    let icon = LoadIconW(instance, 1 as _);
    let window_class = WNDCLASSW {
      hInstance: instance,
      lpszClassName: class.as_ptr(),
      lpfnWndProc: Some(window_proc),
      hCursor: LoadCursorW(null_mut(), IDC_ARROW),
      hIcon: icon,
      hbrBackground: GetSysColorBrush(COLOR_BTNFACE),
      ..Default::default()
    };
    anyhow::ensure!(RegisterClassW(&window_class) != 0, std::io::Error::last_os_error());

    let style = WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
    let mut bounds = RECT {
      left: 0,
      top: 0,
      right: scale(INSTALLER_LAYOUT.width, dpi),
      bottom: scale(INSTALLER_LAYOUT.height, dpi),
    };
    AdjustWindowRectExForDpi(&mut bounds, style, 0, 0, dpi);
    let window = CreateWindowExW(
      0,
      class.as_ptr(),
      wide("DF localization installer").as_ptr(),
      style,
      CW_USEDEFAULT,
      CW_USEDEFAULT,
      bounds.right - bounds.left,
      bounds.bottom - bounds.top,
      null_mut(),
      null_mut(),
      instance,
      null(),
    );
    anyhow::ensure!(!window.is_null(), std::io::Error::last_os_error());

    let ui = Window::new(window, dpi)?;
    let mut app = App::default();
    let mut languages = Vec::new();
    let locale_options = locales(&LOCALE.read().current_locale());
    fill_combo(ui.locale, &locale_options);
    app.on_start();
    ui.refresh(&app, &mut languages, &locale_options);
    anyhow::ensure!(SetTimer(window, 1, 100, None) != 0, std::io::Error::last_os_error());
    ShowWindow(window, SW_SHOWDEFAULT);

    let mut message = MSG::default();
    loop {
      match GetMessageW(&mut message, null_mut(), 0, 0) {
        -1 => return Err(std::io::Error::last_os_error().into()),
        0 => break,
        _ => {}
      }
      if message.hwnd == window && matches!(message.message, WM_TIMER | ACTION) {
        let changed = if message.message == WM_TIMER {
          app.update_state()
        } else {
          ui.action(&mut app, message.wParam, &languages, &locale_options);
          true
        };
        if changed {
          ui.refresh(&app, &mut languages, &locale_options);
          if app.df_running {
            ui.warning(&running_warning(), MB_OK);
            DestroyWindow(window);
          } else if app.ready() && (!app.bin.valid || app.delete_old_data_show) {
            if !app.bin.valid {
              ui.browse(&mut app);
            }
            if IsWindow(window) != 0 {
              ui.confirm_old_data(&mut app);
              ui.refresh(&app, &mut languages, &locale_options);
            }
          }
        }
      } else if IsDialogMessageW(window, &message) == 0 {
        TranslateMessage(&message);
        DispatchMessageW(&message);
      }
    }
    app.on_close();
  }
  Ok(())
}

unsafe extern "system" fn window_proc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
  unsafe {
    match message {
      WM_COMMAND => {
        let id = wparam & 0xffff;
        let notification = (wparam >> 16) as u32;
        let control_event = lparam != 0
          && match id {
            LANGUAGE | LOCALE_SELECT => notification == CBN_SELCHANGE,
            BROWSE | INSTALL | REMOVE => notification == BN_CLICKED,
            BUG_ICON | TRANSIFEX_ICON => notification == STN_CLICKED,
            _ => false,
          };
        if control_event {
          PostMessageW(window, ACTION, id, 0);
        }
        0
      }
      WM_NOTIFY if lparam != 0 && matches!(wparam, BUG_LINK | TRANSIFEX_LINK) => {
        let header = &*(lparam as *const NMHDR);
        if matches!(header.code, NM_CLICK | NM_RETURN) {
          PostMessageW(window, ACTION, wparam, 0);
        }
        0
      }
      WM_SETCURSOR if matches!(GetDlgCtrlID(wparam as HWND) as usize, BUG_ICON | TRANSIFEX_ICON) => {
        SetCursor(LoadCursorW(null_mut(), IDC_HAND));
        1
      }
      WM_CTLCOLORSTATIC => {
        let color = GetWindowLongPtrW(lparam as HWND, GWLP_USERDATA);
        if color == 0 {
          return DefWindowProcW(window, message, wparam, lparam);
        }
        SetTextColor(wparam as HDC, (color - 1) as u32);
        SetBkMode(wparam as HDC, TRANSPARENT as i32);
        GetSysColorBrush(COLOR_BTNFACE) as LRESULT
      }
      WM_DESTROY => {
        KillTimer(window, 1);
        PostQuitMessage(0);
        0
      }
      _ => DefWindowProcW(window, message, wparam, lparam),
    }
  }
}

unsafe extern "system" fn status_bar_proc(
  bar: HWND,
  message: u32,
  wparam: WPARAM,
  lparam: LPARAM,
  _id: usize,
  parent: usize,
) -> LRESULT {
  unsafe {
    // Footer controls notify their immediate parent. Keep routing their actions through
    // the main window, which posts them to the installer's event loop.
    if matches!(message, WM_COMMAND | WM_NOTIFY | WM_SETCURSOR) {
      let result = SendMessageW(parent as HWND, message, wparam, lparam);
      if result != 0 || message != WM_SETCURSOR {
        return result;
      }
    }
    DefSubclassProc(bar, message, wparam, lparam)
  }
}

struct Window {
  handle: HWND,
  font: HFONT,
  heading_font: HFONT,
  dpi: u32,
  path_title: HWND,
  version_title: HWND,
  checksum_title: HWND,
  path: HWND,
  version: HWND,
  platform_icon: HWND,
  platform: HWND,
  checksum: HWND,
  browse: HWND,
  hook_title: HWND,
  hook_version: HWND,
  hook: StatusRow,
  dict_title: HWND,
  dict: StatusRow,
  languages: HWND,
  locale: HWND,
  install: HWND,
  remove: HWND,
  notice: HWND,
  bugs: FooterLink,
  translations: FooterLink,
  build: HWND,
}

impl Window {
  fn new(handle: HWND, dpi: u32) -> anyhow::Result<Self> {
    let mut metrics = NONCLIENTMETRICSW {
      cbSize: size_of::<NONCLIENTMETRICSW>() as u32,
      ..Default::default()
    };
    let font = unsafe {
      if SystemParametersInfoForDpi(
        SPI_GETNONCLIENTMETRICS,
        metrics.cbSize,
        (&mut metrics as *mut NONCLIENTMETRICSW).cast(),
        0,
        dpi,
      ) != 0
      {
        CreateFontIndirectW(&metrics.lfMessageFont)
      } else {
        null_mut()
      }
    };
    let heading_font = if font.is_null() {
      null_mut()
    } else {
      metrics.lfMessageFont.lfHeight -= scale(3, dpi);
      unsafe { CreateFontIndirectW(&metrics.lfMessageFont) }
    };
    // Keep the fonts alive for the lifetime of all the child controls.
    let create_child =
      |parent, class: *const u16, text: &str, style: u32, id: usize, [x, y, width, height]: [i32; 4]| {
        let control = unsafe {
          CreateWindowExW(
            0,
            class,
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            scale(x, dpi),
            scale(y, dpi),
            scale(width, dpi),
            scale(height, dpi),
            parent,
            id as HMENU,
            GetModuleHandleW(null()),
            null(),
          )
        };
        anyhow::ensure!(!control.is_null(), std::io::Error::last_os_error());
        if !font.is_null() {
          unsafe { SendMessageW(control, WM_SETFONT, font as usize, 1) };
        }
        Ok(control)
      };
    let create = |class, text, style, id, bounds| create_child(handle, class, text, style, id, bounds);
    use windows_sys::core::w;
    let label = |bounds| create(w!("STATIC"), "", SS_NOPREFIX, 0, bounds);
    let heading = |text, bounds| {
      let control = create(w!("STATIC"), text, SS_NOPREFIX, 0, bounds)?;
      if !heading_font.is_null() {
        unsafe { SendMessageW(control, WM_SETFONT, heading_font as usize, 1) };
      }
      Ok::<_, anyhow::Error>(control)
    };
    let separator = |y| create(w!("STATIC"), "", SS_ETCHEDHORZ, 0, [9, y, 702, 2]);
    let button = |id, bounds| create(w!("BUTTON"), "", WS_TABSTOP, id, bounds);
    let combo = |id, bounds| {
      create(
        w!("COMBOBOX"),
        "",
        WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST as u32,
        id,
        bounds,
      )
    };
    let status_row = |[installed, available, status]: [ControlId; 3], y_offset| {
      let [x, y, width, height] = control_bounds(status, y_offset, 34);
      Ok::<_, anyhow::Error>(StatusRow {
        local: label(control_bounds(installed, y_offset, 24))?,
        available: label(control_bounds(available, y_offset, 24))?,
        icon: create(w!("STATIC"), "", SS_ICON, 0, [x, y, 16, 16])?,
        status: label([x + 22, y, width - 22, height])?,
      })
    };

    let footer_link = |bar, text_id, icon_id, resource| {
      let icon = create_child(bar, w!("STATIC"), "", SS_ICON | SS_NOTIFY, icon_id, [9, 13, 16, 16])?;
      set_icon(icon, STM_SETICON, resource, scale(16, dpi));
      let text = create_child(bar, WC_LINK, "", WS_TABSTOP, text_id, [30, 15, 250, 22])?;
      Ok::<_, anyhow::Error>(FooterLink { icon, text })
    };

    let result = (|| {
      use ControlId::*;
      heading("Dwarf Fortress", control_bounds(GameDetailsHeading, 0, 22))?;
      separator(row_y(ContentRow::Heading(SectionHeading::GameDetails)) + 25)?;
      let path_title = label(control_bounds(GamePathLabel, 4, 22))?;
      let [x, y, width, height] = control_bounds(GamePathValue, 1, 23);
      let path = create(
        w!("EDIT"),
        "",
        WS_BORDER | WS_TABSTOP | (ES_READONLY | ES_AUTOHSCROLL) as u32,
        0,
        [x, y, width - 6, height],
      )?;
      let browse = button(BROWSE, control_bounds(BrowseExecutable, 0, 25))?;
      set_text(browse, "…");
      let version_title = label(control_bounds(GameVersionLabel, 0, 22))?;
      let version = label(control_bounds(GameVersionValue, 0, 22))?;
      let platform_icon = create(w!("STATIC"), "", SS_ICON, 0, [0, 0, 16, 16])?;
      let platform = label([0, 0, 425, 22])?;
      let checksum_title = label(control_bounds(GameChecksumLabel, 0, 22))?;
      let checksum = label(control_bounds(GameChecksumValue, 0, 22))?;

      let hook_title = heading("", control_bounds(HookHeading, 0, 23))?;
      separator(row_y(ContentRow::Heading(SectionHeading::Hook)) + 28)?;
      let hook_version = label(control_bounds(HookVersionLabel, 0, 24))?;
      let hook = status_row([HookInstalledVersion, HookAvailableVersion, HookUpdateStatus], 0)?;

      let dict_title = heading("", control_bounds(DictionaryHeading, 0, 22))?;
      separator(row_y(ContentRow::Heading(SectionHeading::Dictionary)) + 27)?;
      let [x, y, width, height] = control_bounds(DictionaryLanguageSelector, 0, 190);
      let languages = combo(LANGUAGE, [x, y, width - 11, height])?;
      let dict = status_row(
        [
          DictionaryInstalledVersion,
          DictionaryAvailableVersion,
          DictionaryUpdateStatus,
        ],
        4,
      )?;

      let [x, y, width, height] = control_bounds(InstallOrUpdate, 0, INSTALL_HEIGHT);
      let install = button(INSTALL, [x + (width - INSTALL_WIDTH) / 2, y, INSTALL_WIDTH, height])?;
      let notice = label(control_bounds(OperationNotice, 0, 39))?;
      let bar = create(STATUSCLASSNAMEW, "", WS_CLIPCHILDREN, 0, [0, 0, 0, 0])?;
      unsafe {
        let style = GetWindowLongPtrW(bar, GWL_EXSTYLE);
        SetWindowLongPtrW(bar, GWL_EXSTYLE, style | WS_EX_CONTROLPARENT as isize);
        anyhow::ensure!(
          SetWindowSubclass(bar, Some(status_bar_proc), 1, handle as usize) != 0,
          "Unable to initialize the status bar"
        );
        SendMessageW(bar, SB_SETMINHEIGHT, scale(39, dpi) as WPARAM, 0);
        SendMessageW(bar, WM_SIZE, 0, 0);
      }
      let bugs = footer_link(bar, BUG_LINK, BUG_ICON, GITHUB_RESOURCE)?;
      let translations = footer_link(bar, TRANSIFEX_LINK, TRANSIFEX_ICON, TRANSIFEX_RESOURCE)?;
      let build = create_child(bar, w!("STATIC"), &format!("v{VERSION}"), 0, 0, [535, 15, 75, 22])?;
      let locale = create_child(
        bar,
        w!("COMBOBOX"),
        "",
        WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST as u32,
        LOCALE_SELECT,
        [640, 9, 71, 200],
      )?;
      let remove = create_child(
        bar,
        w!("BUTTON"),
        "",
        WS_TABSTOP | BS_ICON as u32,
        REMOVE,
        [680, 8, 31, 25],
      )?;
      set_icon(remove, BM_SETIMAGE, TRASH_RESOURCE, scale(16, dpi));
      Ok(Self {
        handle,
        font,
        heading_font,
        dpi,
        path_title,
        version_title,
        checksum_title,
        path,
        version,
        platform_icon,
        platform,
        checksum,
        browse,
        hook_title,
        hook_version,
        hook,
        dict_title,
        dict,
        languages,
        locale,
        install,
        remove,
        notice,
        bugs,
        translations,
        build,
      })
    })();
    if result.is_err() {
      unsafe {
        DestroyWindow(handle);
        if !font.is_null() {
          DeleteObject(font);
        }
        if !heading_font.is_null() {
          DeleteObject(heading_font);
        }
      }
    }
    result
  }

  fn refresh(&self, app: &App, languages: &mut Vec<String>, locale_options: &[String]) {
    let view = app.view();
    set_text(self.path_title, &t!("Path"));
    set_text(self.version_title, &t!("Version"));
    set_text(self.checksum_title, &t!("Checksum"));
    set_text(self.path, &view.path);
    set_text(self.version, &view.version);
    set_text(self.platform, &format!("{} | {}", view.platform, view.distribution));
    let resource = match view.platform {
      OS::Linux => LINUX_RESOURCE,
      OS::Windows => WINDOWS_RESOURCE,
    };
    set_icon(self.platform_icon, STM_SETICON, resource, scale(16, self.dpi));
    self.place_version(&view.version, app.bin.valid);
    set_text(self.checksum, &view.checksum);
    set_text(self.hook_title, &t!("Hook"));
    set_text(self.hook_version, &t!("Version"));
    self.hook.refresh(&view.hook, self.dpi);
    set_text(self.dict_title, &t!("Dictionary"));
    self.dict.refresh(&view.dictionary, self.dpi);
    if *languages != view.languages {
      fill_combo(self.languages, &view.languages);
      *languages = view.languages;
    }
    select_combo(self.languages, languages, &app.selected_language);
    select_combo(self.locale, locale_options, &app.ui_locale);
    set_text(self.install, &view.action);
    set_text(self.remove, &t!("Delete localization files"));
    if !app.ready() || view.busy {
      set_text(self.notice, &t!("loading data"));
    } else {
      set_text(self.notice, app.notice.as_deref().unwrap_or_default());
    }
    self.refresh_footer(view.has_localization);
    unsafe {
      EnableWindow(self.browse, (view.ready && !view.busy).into());
      EnableWindow(
        self.languages,
        (view.ready && !view.busy && !languages.is_empty()).into(),
      );
      ShowWindow(self.install, if view.can_update { SW_SHOWNA } else { SW_HIDE });
      ShowWindow(self.remove, if view.has_localization { SW_SHOWNA } else { SW_HIDE });
      EnableWindow(self.remove, (view.ready && !view.busy).into());
    }
  }

  fn place_version(&self, version: &str, show_icon: bool) {
    let width = text_width(self.version, self.font, version) + scale(4, self.dpi);
    let [left, top, available_width, _] = control_bounds(ControlId::GameVersionValue, 0, 22);
    let x = scale(left, self.dpi);
    let icon_size = scale(16, self.dpi);
    let platform_x = x + width + if show_icon { scale(21, self.dpi) } else { 0 };
    unsafe {
      MoveWindow(self.version, x, scale(top, self.dpi), width, scale(22, self.dpi), 1);
      MoveWindow(
        self.platform_icon,
        x + width,
        scale(top - 2, self.dpi),
        icon_size,
        icon_size,
        1,
      );
      ShowWindow(self.platform_icon, if show_icon { SW_SHOWNA } else { SW_HIDE });
      MoveWindow(
        self.platform,
        platform_x,
        scale(top, self.dpi),
        (scale(left + available_width, self.dpi) - platform_x).max(0),
        scale(22, self.dpi),
        1,
      );
    }
  }

  fn refresh_footer(&self, show_remove: bool) {
    let mut left = scale(9, self.dpi);
    for item in INSTALLER_LAYOUT.footer.leading_items.iter().copied() {
      let width = self.footer_width(item, show_remove);
      if width > 0 {
        self.place_footer_item(item, left, width);
        left += width + scale(10, self.dpi);
      }
    }

    let mut right = scale(INSTALLER_LAYOUT.width - 9, self.dpi);
    for item in INSTALLER_LAYOUT.footer.trailing_items.iter().copied().rev() {
      let width = self.footer_width(item, show_remove);
      if width > 0 {
        right -= width;
        self.place_footer_item(item, right, width);
        right -= scale(5, self.dpi);
      }
    }
  }

  fn footer_width(&self, item: FooterItem, show_remove: bool) -> i32 {
    match item {
      FooterItem::ExternalLink(FooterLinkTarget::ReportBug) => self.bugs.measure(FooterLinkTarget::ReportBug, self.dpi),
      FooterItem::ExternalLink(FooterLinkTarget::HelpWithTranslation) => self
        .translations
        .measure(FooterLinkTarget::HelpWithTranslation, self.dpi),
      FooterItem::InstallerVersion => scale(75, self.dpi),
      FooterItem::InterfaceLanguageSelector => scale(71, self.dpi),
      FooterItem::RemoveLocalizationButton => {
        if show_remove {
          scale(31, self.dpi)
        } else {
          0
        }
      }
    }
  }

  fn place_footer_item(&self, item: FooterItem, x: i32, width: i32) {
    let (control, y, height) = match item {
      FooterItem::ExternalLink(FooterLinkTarget::ReportBug) => return self.bugs.place(x, width, self.dpi),
      FooterItem::ExternalLink(FooterLinkTarget::HelpWithTranslation) => {
        return self.translations.place(x, width, self.dpi);
      }
      FooterItem::InstallerVersion => (self.build, 15, 22),
      FooterItem::InterfaceLanguageSelector => (self.locale, 9, 200),
      FooterItem::RemoveLocalizationButton => (self.remove, 8, 25),
    };
    unsafe { MoveWindow(control, x, scale(y, self.dpi), width, scale(height, self.dpi), 1) };
  }

  fn action(&self, app: &mut App, id: usize, languages: &[String], locale_options: &[String]) {
    match id {
      BROWSE => self.browse(app),
      LANGUAGE => {
        if let Some(language) = combo_value(self.languages, languages) {
          app.select_language(language);
        }
      }
      LOCALE_SELECT => {
        if let Some(locale) = combo_value(self.locale, locale_options) {
          app.select_locale(locale);
        }
      }
      INSTALL => app.update_data(),
      REMOVE => {
        if self.warning(&t!("Delete all localization files?"), MB_YESNO | MB_DEFBUTTON2) == IDYES {
          app.confirm_localization_removal();
        }
      }
      BUG_LINK | BUG_ICON => self.open_url(FooterLinkTarget::ReportBug.url()),
      TRANSIFEX_LINK | TRANSIFEX_ICON => self.open_url(FooterLinkTarget::HelpWithTranslation.url()),
      _ => {}
    }
  }

  fn browse(&self, app: &mut App) {
    loop {
      let Some(path) = pick_file(self.handle, &app.bin.dir) else {
        if !app.bin.valid {
          unsafe { DestroyWindow(self.handle) };
        }
        return;
      };
      app.select_binary(path);
      if app.bin.valid {
        return;
      }
      if let Some(text) = &app.notice {
        self.warning(text, MB_OK);
      }
    }
  }

  fn confirm_old_data(&self, app: &mut App) {
    if app.delete_old_data_show {
      app.delete_old_data_show = false;
      if self.warning(&old_data_warning(), MB_YESNO | MB_DEFBUTTON2) == IDYES {
        app.confirm_old_data_removal();
      }
    }
  }

  fn warning(&self, text: &str, buttons: MESSAGEBOX_STYLE) -> MESSAGEBOX_RESULT {
    unsafe {
      MessageBoxW(
        self.handle,
        wide(text).as_ptr(),
        wide(&t!("Warning")).as_ptr(),
        buttons | MB_ICONWARNING,
      )
    }
  }

  fn open_url(&self, url: &str) {
    unsafe { ShellExecuteW(self.handle, null(), wide(url).as_ptr(), null(), null(), SW_SHOWNORMAL) };
  }
}

impl Drop for Window {
  fn drop(&mut self) {
    unsafe {
      if IsWindow(self.handle) != 0 {
        DestroyWindow(self.handle);
      }
      if !self.font.is_null() {
        DeleteObject(self.font);
      }
      if !self.heading_font.is_null() {
        DeleteObject(self.heading_font);
      }
    }
  }
}

struct FooterLink {
  icon: HWND,
  text: HWND,
}

impl FooterLink {
  fn measure(&self, link: FooterLinkTarget, dpi: u32) -> i32 {
    let title = link.title();
    let url = link.url();
    set_text(self.text, &format!(r#"<a href="{url}">{title}</a>"#));
    let mut size = SIZE::default();
    let max_width = scale(260, dpi);
    unsafe {
      SendMessageW(
        self.text,
        LM_GETIDEALSIZE,
        max_width as usize,
        (&mut size as *mut SIZE) as LPARAM,
      );
    }
    size.cx.clamp(1, max_width) + scale(21, dpi)
  }

  fn place(&self, x: i32, width: i32, dpi: u32) {
    let inset = scale(21, dpi);
    unsafe {
      MoveWindow(self.icon, x, scale(13, dpi), scale(16, dpi), scale(16, dpi), 1);
      MoveWindow(self.text, x + inset, scale(15, dpi), width - inset, scale(22, dpi), 1);
    }
  }
}

fn set_icon(control: HWND, message: u32, resource: usize, size: i32) {
  // All the controls use the same system DPI and the shared icon stays valid for the process.
  unsafe {
    let icon = LoadImageW(
      GetModuleHandleW(null()),
      resource as _,
      IMAGE_ICON,
      size,
      size,
      LR_SHARED,
    );
    let image_type = if message == BM_SETIMAGE {
      IMAGE_ICON as usize
    } else {
      icon as usize
    };
    let image = if message == BM_SETIMAGE { icon as LPARAM } else { 0 };
    SendMessageW(control, message, image_type, image);
  }
}

struct StatusRow {
  local: HWND,
  available: HWND,
  icon: HWND,
  status: HWND,
}

impl StatusRow {
  fn refresh(&self, value: &ComponentStatus, dpi: u32) {
    set_text(self.local, &value.local);
    set_text(self.available, &value.available);
    let background = unsafe { GetSysColor(COLOR_BTNFACE) };
    let (red, green, blue) = value.status.color((background >> 8) & 0xff < 128);
    let color = u32::from(red) | (u32::from(green) << 8) | (u32::from(blue) << 16);
    unsafe {
      SetWindowLongPtrW(self.status, GWLP_USERDATA, color as isize + 1);
      InvalidateRect(self.status, null(), 1);
    }
    let resource = match value.status {
      Status::Loading => 5,
      Status::Error => 6,
      Status::Current => 7,
      Status::Attention => 8,
    };
    set_icon(self.icon, STM_SETICON, resource, scale(16, dpi));
    set_text(self.status, &value.text);
  }
}

fn pick_file(window: HWND, directory: &Path) -> Option<PathBuf> {
  let mut buffer = vec![0u16; 32768];
  let title = wide(&t!("Open Dwarf Fortress executable"));
  let directory: Vec<_> = directory.as_os_str().encode_wide().chain([0]).collect();
  let mut dialog = OPENFILENAMEW {
    lStructSize: size_of::<OPENFILENAMEW>() as u32,
    hwndOwner: window,
    lpstrFilter: windows_sys::core::w!("Executable (*.exe)\0*.exe\0"),
    lpstrFile: buffer.as_mut_ptr(),
    nMaxFile: buffer.len() as u32,
    lpstrInitialDir: directory.as_ptr(),
    lpstrTitle: title.as_ptr(),
    Flags: OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
    ..Default::default()
  };
  if unsafe { GetOpenFileNameW(&mut dialog) } == 0 {
    return None;
  }
  let end = buffer.iter().position(|unit| *unit == 0)?;
  Some(OsString::from_wide(&buffer[..end]).into())
}

fn fill_combo(control: HWND, options: &[String]) {
  unsafe {
    SendMessageW(control, CB_RESETCONTENT, 0, 0);
    for option in options {
      SendMessageW(control, CB_ADDSTRING, 0, wide(option).as_ptr() as LPARAM);
    }
  }
}

fn select_combo(control: HWND, options: &[String], selected: &str) {
  let index = options
    .iter()
    .position(|option| option == selected)
    .unwrap_or(usize::MAX);
  unsafe { SendMessageW(control, CB_SETCURSEL, index, 0) };
}

fn combo_value(control: HWND, options: &[String]) -> Option<String> {
  let index = unsafe { SendMessageW(control, CB_GETCURSEL, 0, 0) };
  options.get(index as usize).cloned()
}

fn text_width(control: HWND, font: HFONT, text: &str) -> i32 {
  let text = wide(text);
  let mut size = SIZE::default();
  unsafe {
    let dc = GetDC(control);
    if dc.is_null() {
      return 0;
    }
    let previous = if font.is_null() {
      null_mut()
    } else {
      SelectObject(dc, font)
    };
    GetTextExtentPoint32W(dc, text.as_ptr(), (text.len() - 1) as i32, &mut size);
    if !previous.is_null() {
      SelectObject(dc, previous);
    }
    ReleaseDC(control, dc);
  }
  size.cx
}

fn set_text(control: HWND, text: &str) {
  unsafe { SetWindowTextW(control, wide(text).as_ptr()) };
}

fn row_y(row: ContentRow) -> i32 {
  // Native row heights include room for the controls and the gap before the next row.
  12 + INSTALLER_LAYOUT
    .content_rows()
    .take_while(|&item| item != row)
    .map(|item| match item {
      ContentRow::Heading(SectionHeading::GameDetails) => 32,
      ContentRow::SectionRow(SectionRow::GameExecutablePath) => 33,
      ContentRow::SectionRow(SectionRow::GameVersion) => 24,
      ContentRow::SectionRow(SectionRow::GameChecksum) => 39,
      ContentRow::Heading(SectionHeading::Hook) => 38,
      ContentRow::SectionRow(SectionRow::HookVersionStatus) => 40,
      ContentRow::Heading(SectionHeading::Dictionary) => 35,
      ContentRow::SectionRow(SectionRow::DictionaryLanguageAndStatus) => 53,
      ContentRow::InstallOrUpdateAction => 56,
      ContentRow::OperationNotice => 39,
    })
    .sum::<i32>()
}

fn control_bounds(control: ControlId, y_offset: i32, height: i32) -> [i32; 4] {
  const EDGES: [i32; ContentColumn::COUNT as usize + 1] = [9, 165, 320, 475, 676, 711];
  let (row, cell) = INSTALLER_LAYOUT
    .content_rows()
    .find_map(|row| {
      row
        .cells()
        .iter()
        .find(|cell| cell.control == control)
        .map(|cell| (row, cell))
    })
    .unwrap_or_else(|| panic!("Missing layout cell for {control:?}"));
  let column = cell.column.index() as usize;
  let left = EDGES[column];
  let right = EDGES[column + cell.column_span as usize];
  [left, row_y(row) + y_offset, right - left, height]
}

fn scale(value: i32, dpi: u32) -> i32 {
  (value * dpi as i32 + 48) / 96
}

fn wide(text: &str) -> Vec<u16> {
  text.encode_utf16().chain([0]).collect()
}
