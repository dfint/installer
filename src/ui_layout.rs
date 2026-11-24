//! Shared window structure. Each backend supplies the native controls and spacing.

use crate::{
  constants::{URL_BUGS, URL_TRANSIFEX},
  localization::{LOCALE, t},
};

pub const WINDOW_WIDTH: i32 = 720;
pub const WINDOW_HEIGHT: i32 = 450;
pub const INSTALL_WIDTH: i32 = 130;
pub const INSTALL_HEIGHT: i32 = 40;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ContentRow {
  GameDetailsHeading,
  GameExecutablePath,
  GameVersion,
  GameChecksum,
  HookHeading,
  HookVersionStatus,
  DictionaryHeading,
  DictionaryLanguageAndStatus,
  InstallOrUpdateAction,
  OperationNotice,
}

pub const CONTENT_ROWS: [ContentRow; 10] = [
  ContentRow::GameDetailsHeading,
  ContentRow::GameExecutablePath,
  ContentRow::GameVersion,
  ContentRow::GameChecksum,
  ContentRow::HookHeading,
  ContentRow::HookVersionStatus,
  ContentRow::DictionaryHeading,
  ContentRow::DictionaryLanguageAndStatus,
  ContentRow::InstallOrUpdateAction,
  ContentRow::OperationNotice,
];

#[derive(Clone, Copy)]
pub enum ContentColumn {
  FieldLabel,
  PrimaryValue,
  AvailableVersion,
  UpdateStatus,
  BrowseExecutableButton,
}

impl ContentColumn {
  pub const COUNT: i32 = 5;

  pub fn index(self) -> i32 {
    self as i32
  }
}

#[derive(Clone, Copy)]
pub enum FooterLinkTarget {
  ReportBug,
  HelpWithTranslation,
}

impl FooterLinkTarget {
  pub fn url(self) -> &'static str {
    match self {
      Self::ReportBug => URL_BUGS,
      Self::HelpWithTranslation => URL_TRANSIFEX,
    }
  }

  pub fn title(self) -> String {
    match self {
      Self::ReportBug => t!("Report bug"),
      Self::HelpWithTranslation => t!("Help with translation"),
    }
  }
}

#[derive(Clone, Copy)]
pub enum FooterItem {
  ExternalLink(FooterLinkTarget),
  InstallerVersion,
  InterfaceLanguageSelector,
  RemoveLocalizationButton,
}

pub const FOOTER_LEADING_ITEMS: [FooterItem; 3] = [
  FooterItem::ExternalLink(FooterLinkTarget::ReportBug),
  FooterItem::ExternalLink(FooterLinkTarget::HelpWithTranslation),
  FooterItem::InstallerVersion,
];
pub const FOOTER_TRAILING_ITEMS: [FooterItem; 2] = [
  FooterItem::InterfaceLanguageSelector,
  FooterItem::RemoveLocalizationButton,
];
