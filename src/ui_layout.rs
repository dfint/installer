//! Shared window structure. Each backend supplies the native controls and spacing.

use crate::{
  constants::{BUG_REPORT_URL, TRANSLATION_HELP_URL},
  localization::{LOCALE, t},
};

pub const INSTALL_WIDTH: i32 = 130;
pub const INSTALL_HEIGHT: i32 = 40;

pub const INSTALLER_LAYOUT: WindowLayout = WindowLayout {
  width: 720,
  height: 450,
  content: &[
    ContentItem::Section(Section {
      heading: SectionHeading::GameDetails,
      rows: &[
        SectionRow::GameExecutablePath,
        SectionRow::GameVersion,
        SectionRow::GameChecksum,
      ],
    }),
    ContentItem::Section(Section {
      heading: SectionHeading::Hook,
      rows: &[SectionRow::HookVersionStatus],
    }),
    ContentItem::Section(Section {
      heading: SectionHeading::Dictionary,
      rows: &[SectionRow::DictionaryLanguageAndStatus],
    }),
    ContentItem::InstallOrUpdateAction,
    ContentItem::OperationNotice,
  ],
  footer: FooterLayout {
    leading_items: &[
      FooterItem::ExternalLink(FooterLinkTarget::ReportBug),
      FooterItem::ExternalLink(FooterLinkTarget::HelpWithTranslation),
      FooterItem::InstallerVersion,
    ],
    trailing_items: &[
      FooterItem::InterfaceLanguageSelector,
      FooterItem::RemoveLocalizationButton,
    ],
  },
};

pub struct WindowLayout {
  pub width: i32,
  pub height: i32,
  content: &'static [ContentItem],
  pub footer: FooterLayout,
}

impl WindowLayout {
  /// The native renderers share column alignment across section boundaries.
  pub fn content_rows(&self) -> impl Iterator<Item = ContentRow> + '_ {
    self.content.iter().flat_map(|item| {
      let (first_row, section_rows) = match item {
        ContentItem::Section(section) => (ContentRow::Heading(section.heading), section.rows),
        ContentItem::InstallOrUpdateAction => (ContentRow::InstallOrUpdateAction, &[][..]),
        ContentItem::OperationNotice => (ContentRow::OperationNotice, &[][..]),
      };
      std::iter::once(first_row).chain(section_rows.iter().copied().map(ContentRow::SectionRow))
    })
  }
}

enum ContentItem {
  Section(Section),
  InstallOrUpdateAction,
  OperationNotice,
}

struct Section {
  heading: SectionHeading,
  rows: &'static [SectionRow],
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SectionHeading {
  GameDetails,
  Hook,
  Dictionary,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SectionRow {
  GameExecutablePath,
  GameVersion,
  GameChecksum,
  HookVersionStatus,
  DictionaryLanguageAndStatus,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ContentRow {
  Heading(SectionHeading),
  SectionRow(SectionRow),
  InstallOrUpdateAction,
  OperationNotice,
}

pub struct FooterLayout {
  pub leading_items: &'static [FooterItem],
  pub trailing_items: &'static [FooterItem],
}

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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum FooterLinkTarget {
  ReportBug,
  HelpWithTranslation,
}

impl FooterLinkTarget {
  pub fn url(self) -> &'static str {
    match self {
      Self::ReportBug => BUG_REPORT_URL,
      Self::HelpWithTranslation => TRANSLATION_HELP_URL,
    }
  }

  pub fn title(self) -> String {
    match self {
      Self::ReportBug => t!("Report bug"),
      Self::HelpWithTranslation => t!("Help with translation"),
    }
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum FooterItem {
  ExternalLink(FooterLinkTarget),
  InstallerVersion,
  InterfaceLanguageSelector,
  RemoveLocalizationButton,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ControlId {
  GameDetailsHeading,
  GamePathLabel,
  GamePathValue,
  BrowseExecutable,
  GameVersionLabel,
  GameVersionValue,
  GameChecksumLabel,
  GameChecksumValue,
  HookHeading,
  HookVersionLabel,
  HookInstalledVersion,
  HookAvailableVersion,
  HookUpdateStatus,
  DictionaryHeading,
  DictionaryLanguageSelector,
  DictionaryInstalledVersion,
  DictionaryAvailableVersion,
  DictionaryUpdateStatus,
  InstallOrUpdate,
  OperationNotice,
}

pub struct Cell {
  pub control: ControlId,
  pub column: ContentColumn,
  pub column_span: i32,
}

impl Cell {
  const fn new(control: ControlId, column: ContentColumn) -> Self {
    Self {
      control,
      column,
      column_span: 1,
    }
  }

  const fn span(mut self, columns: i32) -> Self {
    self.column_span = columns;
    self
  }
}

impl ContentRow {
  pub fn cells(self) -> &'static [Cell] {
    use ContentColumn::{AvailableVersion, BrowseExecutableButton, FieldLabel, PrimaryValue, UpdateStatus};
    use ControlId::*;
    match self {
      Self::Heading(SectionHeading::GameDetails) => {
        const { &[Cell::new(GameDetailsHeading, FieldLabel).span(ContentColumn::COUNT)] }
      }
      Self::Heading(SectionHeading::Hook) => const { &[Cell::new(HookHeading, FieldLabel).span(ContentColumn::COUNT)] },
      Self::Heading(SectionHeading::Dictionary) => {
        const { &[Cell::new(DictionaryHeading, FieldLabel).span(ContentColumn::COUNT)] }
      }
      Self::SectionRow(SectionRow::GameExecutablePath) => {
        const {
          &[
            Cell::new(GamePathLabel, FieldLabel),
            Cell::new(GamePathValue, PrimaryValue).span(3),
            Cell::new(BrowseExecutable, BrowseExecutableButton),
          ]
        }
      }
      Self::SectionRow(SectionRow::GameVersion) => {
        const {
          &[
            Cell::new(GameVersionLabel, FieldLabel),
            Cell::new(GameVersionValue, PrimaryValue).span(4),
          ]
        }
      }
      Self::SectionRow(SectionRow::GameChecksum) => {
        const {
          &[
            Cell::new(GameChecksumLabel, FieldLabel),
            Cell::new(GameChecksumValue, PrimaryValue).span(4),
          ]
        }
      }
      Self::SectionRow(SectionRow::HookVersionStatus) => {
        const {
          &[
            Cell::new(HookVersionLabel, FieldLabel),
            Cell::new(HookInstalledVersion, PrimaryValue),
            Cell::new(HookAvailableVersion, AvailableVersion),
            Cell::new(HookUpdateStatus, UpdateStatus).span(2),
          ]
        }
      }
      Self::SectionRow(SectionRow::DictionaryLanguageAndStatus) => {
        const {
          &[
            Cell::new(DictionaryLanguageSelector, FieldLabel),
            Cell::new(DictionaryInstalledVersion, PrimaryValue),
            Cell::new(DictionaryAvailableVersion, AvailableVersion),
            Cell::new(DictionaryUpdateStatus, UpdateStatus).span(2),
          ]
        }
      }
      Self::InstallOrUpdateAction => const { &[Cell::new(InstallOrUpdate, FieldLabel).span(ContentColumn::COUNT)] },
      Self::OperationNotice => const { &[Cell::new(OperationNotice, FieldLabel).span(ContentColumn::COUNT)] },
    }
  }
}
