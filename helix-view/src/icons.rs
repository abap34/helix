use arc_swap::ArcSwap;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display, path::Path};

use smartstring::{LazyCompact, SmartString};

use crate::theme::Color;

type String = SmartString<LazyCompact>;

/// Centralized location for icons that can be used throughout the UI.
pub static ICONS: Lazy<ArcSwap<Icons>> = Lazy::new(ArcSwap::default);

/// Centralized location for icons that can be used throughout the UI.
///
/// ```no_run
/// use helix_view::icons::ICONS;
/// use std::path::Path;
///
/// let icons = ICONS.load();
///
/// assert_eq!("󱘗", icons.fs().from_path(Path::new("test.rs")).unwrap().glyph());
/// ```
#[derive(Debug, Default, Deserialize, PartialEq, Eq, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Icons {
    fs: Fs,
    kind: Kind,
    diagnostic: Diagnostic,
    vcs: Vcs,
    dap: Dap,
    ui: Ui,
}

impl Icons {
    /// Returns a handle to all filesystem related icons.
    ///
    /// ```no_run
    /// use helix_view::icons::ICONS;
    /// use std::path::Path;
    ///
    /// let icons = ICONS.load();
    ///
    /// assert_eq!("󱘗", icons.fs().from_path(Path::new("test.rs")).unwrap().glyph());
    /// ```
    #[inline]
    pub fn fs(&self) -> &Fs {
        &self.fs
    }

    /// Returns a handle to all symbol and completion icons.
    ///
    /// ```no_run
    /// use helix_view::icons::ICONS;
    ///
    /// let icons = ICONS.load();
    ///
    /// assert_eq!("■", icons.kind().color().glyph());
    /// assert_eq!("", icons.kind().word().unwrap().glyph());
    /// ```
    #[inline]
    pub fn kind(&self) -> &Kind {
        &self.kind
    }

    /// Returns a handle to all diagnostic related icons.
    ///
    /// ```
    /// use helix_view::icons::ICONS;
    ///
    /// let icons = ICONS.load();
    ///
    /// assert_eq!("▲", icons.diagnostic().warning());
    /// assert_eq!("■", icons.diagnostic().error());
    /// ```
    #[inline]
    pub fn diagnostic(&self) -> &Diagnostic {
        &self.diagnostic
    }

    /// Returns a handle to all version control related icons.
    ///
    /// ```no_run
    /// use helix_view::icons::ICONS;
    ///
    /// let icons = ICONS.load();
    ///
    /// assert_eq!("", icons.vcs().branch().unwrap());
    /// ```
    #[inline]
    pub fn vcs(&self) -> &Vcs {
        &self.vcs
    }

    /// Returns a handle to all debug related icons.
    ///
    /// ```
    /// use helix_view::icons::ICONS;
    ///
    /// let icons = ICONS.load();
    ///
    /// assert_eq!("●", icons.dap().verified());
    /// assert_eq!("◯", icons.dap().unverified());
    /// assert_eq!("▶", icons.dap().play());
    /// ```
    #[inline]
    pub fn dap(&self) -> &Dap {
        &self.dap
    }

    /// Returns a handle to all UI related icons.
    ///
    /// These icons relate to things like virtual text and statusline elements, visual elements, rather than some other
    /// well defined group.
    ///
    /// ```
    /// use helix_view::icons::ICONS;
    ///
    /// let icons = ICONS.load();
    ///
    /// assert_eq!("W", icons.ui().workspace().glyph());
    /// assert_eq!(" ", icons.ui().r#virtual().ruler());
    /// assert_eq!("│", icons.ui().statusline().separator());
    /// ```
    #[inline]
    pub fn ui(&self) -> &Ui {
        &self.ui
    }
}

macro_rules! iconmap {
    ( $( $key:literal => { glyph: $glyph:expr $(, color: $color:expr)? } ),* $(,)? ) => {{
        HashMap::from(
            [
                $(
                  (String::from($key), Icon {
                    glyph: String::from($glyph),
                    color: None $(.or( Some(Color::from_hex($color).unwrap())) )?,
                  }),
                )*
            ]
        )
    }};
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Icon {
    glyph: String,
    color: Option<Color>,
}

impl Icon {
    pub fn glyph(&self) -> &str {
        self.glyph.as_str()
    }

    pub const fn color(&self) -> Option<Color> {
        self.color
    }
}

impl From<&str> for Icon {
    fn from(icon: &str) -> Self {
        Self {
            glyph: String::from(icon),
            color: None,
        }
    }
}

impl Display for Icon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.glyph)
    }
}

impl<'de> Deserialize<'de> for Icon {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(IconVisitor)
    }
}

struct IconVisitor;

impl<'de> serde::de::Visitor<'de> for IconVisitor {
    type Value = Icon;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "a string glyph or a map with 'glyph' and optional 'color'"
        )
    }

    fn visit_str<E>(self, glyph: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Icon {
            glyph: String::from(glyph),
            color: None,
        })
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        let mut glyph = None;
        let mut color = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "glyph" => {
                    if glyph.is_some() {
                        return Err(serde::de::Error::duplicate_field("glyph"));
                    }
                    glyph = Some(map.next_value::<String>()?);
                }
                "color" => {
                    if color.is_some() {
                        return Err(serde::de::Error::duplicate_field("color"));
                    }
                    color = Some(map.next_value::<String>()?);
                }
                _ => return Err(serde::de::Error::unknown_field(&key, &["glyph", "color"])),
            }
        }

        let glyph = glyph.ok_or_else(|| serde::de::Error::missing_field("glyph"))?;

        let color = if let Some(hex) = color {
            let color = Color::from_hex(&hex)
                .map_err(|_| serde::de::Error::custom(format!("`{hex}` is not a valid color code")))?;
            Some(color)
        } else {
            None
        };

        Ok(Icon { glyph, color })
    }
}

#[derive(Debug, Deserialize, Default, PartialEq, Eq, Clone)]
pub struct Kind {
    enabled: bool,

    file: Option<Icon>,
    folder: Option<Icon>,
    text: Option<Icon>,
    module: Option<Icon>,
    namespace: Option<Icon>,
    package: Option<Icon>,
    class: Option<Icon>,
    method: Option<Icon>,
    property: Option<Icon>,
    field: Option<Icon>,
    constructor: Option<Icon>,
    #[serde(rename = "enum")]
    r#enum: Option<Icon>,
    interface: Option<Icon>,
    function: Option<Icon>,
    variable: Option<Icon>,
    constant: Option<Icon>,
    string: Option<Icon>,
    number: Option<Icon>,
    boolean: Option<Icon>,
    array: Option<Icon>,
    object: Option<Icon>,
    key: Option<Icon>,
    null: Option<Icon>,
    enum_member: Option<Icon>,
    #[serde(rename = "struct")]
    r#struct: Option<Icon>,
    event: Option<Icon>,
    operator: Option<Icon>,
    type_parameter: Option<Icon>,
    color: Option<Icon>,
    keyword: Option<Icon>,
    value: Option<Icon>,
    snippet: Option<Icon>,
    reference: Option<Icon>,
    unit: Option<Icon>,
    word: Option<Icon>,
    spellcheck: Option<Icon>,
}

impl Kind {
    #[inline]
    #[must_use]
    pub fn get(&self, kind: &str) -> Option<Icon> {
        if !self.enabled {
            return None;
        }

        match kind {
            "file" => self.file(),
            "folder" => self.folder(),
            "module" => self.module(),
            "namespace" => self.namespace(),
            "package" => self.package(),
            "class" => self.class(),
            "method" => self.method(),
            "property" => self.property(),
            "field" => self.field(),
            "construct" => self.constructor(),
            "enum" => self.r#enum(),
            "interface" => self.interface(),
            "function" => self.function(),
            "variable" => self.variable(),
            "constant" => self.constant(),
            "string" => self.string(),
            "number" => self.number(),
            "boolean" => self.boolean(),
            "array" => self.array(),
            "object" => self.object(),
            "key" => self.key(),
            "null" => self.null(),
            "enum_member" => self.enum_member(),
            "struct" => self.r#struct(),
            "event" => self.event(),
            "operator" => self.operator(),
            "typeparam" => self.type_parameter(),
            "color" => Some(self.color()),
            "keyword" => self.keyword(),
            "value" => self.value(),
            "snippet" => self.snippet(),
            "reference" => self.reference(),
            "text" => self.text(),
            "unit" => self.unit(),
            "word" => self.word(),
            "spellcheck" => self.spellcheck(),

            _ => None,
        }
    }

    #[inline]
    pub fn file(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.file.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn folder(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.folder.clone().or_else(|| Some(Icon::from("󰉋")))
    }

    #[inline]
    pub fn module(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.module.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn namespace(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.namespace.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn package(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.package.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn class(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.class.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn method(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.method.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn property(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.property.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn field(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.field.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn constructor(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.constructor.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn r#enum(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.r#enum.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn interface(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.interface.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn function(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.function.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn variable(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.variable.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn constant(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.constant.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn string(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.string.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn number(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.number.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn boolean(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.boolean.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn array(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.array.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn object(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.object.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn key(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.key.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn null(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.null.clone().or_else(|| Some(Icon::from("󰟢")))
    }

    #[inline]
    pub fn enum_member(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.enum_member.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn r#struct(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.r#struct.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn event(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.event.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn operator(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.operator.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn type_parameter(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.type_parameter
            .clone()
            .or_else(|| Some(Icon::from("")))
    }

    // Always enabled
    #[inline]
    pub fn color(&self) -> Icon {
        self.color.clone().unwrap_or_else(|| Icon::from("■"))
    }

    #[inline]
    pub fn keyword(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.keyword.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn value(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.value.clone().or_else(|| Some(Icon::from("󰎠")))
    }

    #[inline]
    pub fn snippet(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.snippet.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn reference(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.reference.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn text(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.text.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn unit(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.unit.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn word(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.word.clone().or_else(|| Some(Icon::from("")))
    }

    #[inline]
    pub fn spellcheck(&self) -> Option<Icon> {
        if !self.enabled {
            return None;
        }
        self.spellcheck.clone().or_else(|| Some(Icon::from("󰓆")))
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Diagnostic {
    hint: Option<String>,
    info: Option<String>,
    warning: Option<String>,
    error: Option<String>,
}

impl Diagnostic {
    #[inline]
    pub fn hint(&self) -> &str {
        self.hint.as_deref().unwrap_or("○")
    }

    #[inline]
    pub fn info(&self) -> &str {
        self.info.as_deref().unwrap_or("●")
    }

    #[inline]
    pub fn warning(&self) -> &str {
        self.warning.as_deref().unwrap_or("▲")
    }

    #[inline]
    pub fn error(&self) -> &str {
        self.error.as_deref().unwrap_or("■")
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Vcs {
    enabled: bool,
    branch: Option<String>,
    added: Option<String>,
    removed: Option<String>,
    ignored: Option<String>,
    modified: Option<String>,
    renamed: Option<String>,
    conflict: Option<String>,
}

impl Vcs {
    #[inline]
    pub fn branch(&self) -> Option<&str> {
        if !self.enabled {
            return None;
        }
        self.branch.as_deref().or(Some(""))
    }

    #[inline]
    pub fn added(&self) -> Option<&str> {
        if !self.enabled {
            return None;
        }
        self.added.as_deref().or(Some(""))
    }

    #[inline]
    pub fn removed(&self) -> Option<&str> {
        if !self.enabled {
            return None;
        }
        self.removed.as_deref().or(Some(""))
    }

    #[inline]
    pub fn ignored(&self) -> Option<&str> {
        if !self.enabled {
            return None;
        }
        self.ignored.as_deref().or(Some(""))
    }

    #[inline]
    pub fn modified(&self) -> Option<&str> {
        if !self.enabled {
            return None;
        }
        self.modified.as_deref().or(Some(""))
    }

    #[inline]
    pub fn renamed(&self) -> Option<&str> {
        if !self.enabled {
            return None;
        }
        self.renamed.as_deref().or(Some(""))
    }

    #[inline]
    pub fn conflict(&self) -> Option<&str> {
        if !self.enabled {
            return None;
        }
        self.conflict.as_deref().or(Some(""))
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Fs {
    enabled: bool,
    directory: Option<String>,
    #[serde(rename = "directory-open")]
    directory_open: Option<String>,
    #[serde(flatten)]
    mime: HashMap<String, Icon>,
}

static MIMES: once_cell::sync::Lazy<HashMap<String, Icon>> = once_cell::sync::Lazy::new(|| {
    iconmap! {
        // Language name
        "git-commit" => {glyph: "", color: "#F15233" },
        "git-rebase" => {glyph: "", color: "#F15233" },
        "git-config" => {glyph: "", color: "#F15233" },
        "helm" => {glyph: "", color: "#277A9F" },
        "nginx" => {glyph: "", color: "#019639" },
        "rust" => {glyph: "󱘗", color: "#DEA584" },
        "python" => {glyph: "󰌠", color: "#3776AB" },
        "javascript" => {glyph: "󰌞", color: "#F7DF1E" },
        "typescript" => {glyph: "󰛦", color: "#3178C6" },
        "c-sharp" => {glyph: "", color: "#512BD4" },
        "kotlin" => {glyph: "󱈙", color: "#7F52FF" },
        "julia" => {glyph: "", color: "#9558B2" },
        "dockerfile" => {glyph: "󰡨", color: "#2496ED" },
        "text" => { glyph: "" },

        // Exact
        "README.md" => { glyph: "󰂺", color: "#519ABA" },
        "LICENSE" => { glyph: "󰗑", color: "#E7A933" },
        "LICENSE-MIT" => { glyph: "󰗑", color: "#E7A933" },
        "LICENSE-APACHE" => { glyph: "󰗑", color: "#E7A933" },
        "LICENSE-GPL" => { glyph: "󰗑", color: "#E7A933" },
        "LICENSE-AGPL" => { glyph: "󰗑", color: "#E7A933" },
        "CHANGELOG.md" => { glyph: "", color: "#7BAB43" },
        "CODE_OF_CONDUCT.md" => { glyph: "", color: "#F7769D" },
        ".gitignore" => { glyph: "", color: "#F15233" },
        ".gitattributes" => { glyph: "", color: "#F15233" },
        ".git-blame-ignore-revs" => { glyph: "", color: "#F15233" },
        ".gitmodules" => { glyph: "", color: "#F15233" },
        ".editorconfig" => { glyph: "", color: "#FE7743" },
        ".dockerignore" => {glyph: "󰡨", color: "#2496ED" },
        ".ignore" => {glyph: "󰈉", color: "#6C7086" },
        "docker-compose.yaml" => {glyph: "󰡨", color: "#2496ED" },
        "docker-compose.yml" => {glyph: "󰡨", color: "#2496ED" },
        "compose.yaml" => {glyph: "󰡨", color: "#2496ED" },
        "compose.yml" => {glyph: "󰡨", color: "#2496ED" },
        "Cargo.toml" => {glyph: "󱘗", color: "#DEA584" },
        "Cargo.lock" => {glyph: "", color: "#DEA584" },
        "package.json" => {glyph: "󰌞", color: "#F7DF1E" },
        "package-lock.json" => {glyph: "", color: "#CB3837" },
        "pnpm-lock.yaml" => {glyph: "", color: "#F69220" },
        "yarn.lock" => {glyph: "", color: "#2C8EBB" },
        "tsconfig.json" => {glyph: "󰛦", color: "#3178C6" },
        "pyproject.toml" => {glyph: "󰌠", color: "#3776AB" },
        "requirements.txt" => {glyph: "󰌠", color: "#3776AB" },
        "poetry.lock" => {glyph: "", color: "#60A5FA" },
        "go.mod" => {glyph: "󰟓", color: "#00ADD8" },
        "go.sum" => {glyph: "", color: "#00ADD8" },
        "Gemfile" => {glyph: "󰴭", color: "#CC342D" },
        "Gemfile.lock" => {glyph: "", color: "#CC342D" },
        "composer.json" => {glyph: "󰌟", color: "#777BB4" },
        "composer.lock" => {glyph: "", color: "#777BB4" },
        "Makefile" => {glyph: "", color: "#6D8086" },
        ".prettierrc" => {glyph: "", color: "#F7B93E" },
        ".prettierignore" => {glyph: "", color: "#F7B93E" },
        "Dockerfile" => {glyph: "󰡨", color: "#2496ED" },
        ".env" => { glyph: "", color: "#6BA539" },
        ".envrc" => { glyph: "", color: "#6BA539" },
        ".mailmap" => { glyph: "" },
        ".vimrc" => { glyph: "", color: "#007F00" },

        // Extension
        "rs" => {glyph: "󱘗", color: "#DEA584" },
        "py" => {glyph: "󰌠", color: "#3776AB" },
        "pyi" => {glyph: "󰌠", color: "#3776AB" },
        "pyw" => {glyph: "󰌠", color: "#3776AB" },
        "c" => {glyph: "", color: "#A8B9CC" },
        "cpp" => {glyph: "", color: "#659AD2" },
        "cs" => {glyph: "", color: "#512BD4" },
        "d" => {glyph: "", color: "#B03931" },
        "ex" => {glyph: "", color: "#6E4A7E" },
        "exs" => {glyph: "", color: "#6E4A7E" },
        "fs" => {glyph: "", color: "#378BBA" },
        "fsx" => {glyph: "", color: "#378BBA" },
        "go" => {glyph: "󰟓", color: "#00ADD8" },
        "hs" => {glyph: "󰲒", color: "#5D4F85" },
        "java" => {glyph: "󰬷", color: "#ED8B00" },
        "js" => {glyph: "󰌞", color: "#F7DF1E" },
        "mjs" => {glyph: "󰌞", color: "#F7DF1E" },
        "cjs" => {glyph: "󰌞", color: "#F7DF1E" },
        "ts" => {glyph: "󰛦", color: "#3178C6" },
        "mts" => {glyph: "󰛦", color: "#3178C6" },
        "cts" => {glyph: "󰛦", color: "#3178C6" },
        "kt" => {glyph: "󱈙", color: "#7F52FF" },
        "html" => {glyph: "󰌝", color: "#E34F26" },
        "css" => {glyph: "󰌜", color: "#1572B6" },
        "scss" => {glyph: "󰟬", color: "#CC6699" },
        "sh" => {glyph: "", color: "#89E051" },
        "bash" => {glyph: "", color: "#89E051" },
        "nu" => {glyph: "", color: "#8CC84B" },
        "zsh" => {glyph: "", color: "#89E051" },
        "fish" => {glyph: "", color: "#4AAE46" },
        "cmd" => {glyph: "", color: "#C05334" },
        "elv" => {glyph: "", color: "#89E051" },
        "php" => {glyph: "󰌟", color: "#777BB4" },
        "ps1" => {glyph: "󰨊", color: "#5391FE" },
        "dart" => {glyph: "", color: "#0175C2" },
        "rb" => {glyph: "󰴭", color: "#CC342D" },
        "ruby" => {glyph: "󰴭", color: "#CC342D" },
        "swift" => {glyph: "󰛥", color: "#F05138" },
        "r" => {glyph: "󰟔", color: "#276DC3" },
        "groovy" => {glyph: "", color: "#4298B8" },
        "scala" => {glyph: "", color: "#DC322F" },
        "pl" => {glyph: "", color: "#39457E" },
        "clj" => {glyph: "", color: "#5881D8" },
        "jl" => {glyph: "", color: "#9558B2" },
        "zig" => {glyph: "", color: "#F7A41D" },
        "f" => {glyph: "󱈚", color: "#734F96" },
        "erl" => {glyph: "", color: "#A90533" },
        "ml" => {glyph: "", color: "#EC6813" },
        "cr" => {glyph: "", color: "#C8C8C8" },
        "svelte" => {glyph: "", color: "#FF3E00" },
        "gd" => {glyph: "", color: "#478CBF" },
        "nim" => {glyph: "", color: "#EFC743" },
        "jsx" => {glyph: "", color: "#61DAFB" },
        "tsx" => {glyph: "", color: "#61DAFB" },
        "twig" => {glyph: "", color: "#8BC34A" },
        "lua" => {glyph: "", color: "#2C2D72" },
        "vue" => {glyph: "", color: "#42B883" },
        "lisp" => {glyph: "" },
        "elm" => {glyph: "", color: "#1293D8" },
        "res" => {glyph: "", color: "#E6484F" },
        "sol" => {glyph: "", color: "#6366F1" },
        "vala" => {glyph: "", color: "#A972E4" },
        "scm" => {glyph: "", color: "#D53D32" },
        "v" => {glyph: "", color: "#5E87C0" },
        "prisma" => {glyph: "" },
        "ada" => {glyph: "", color: "#195C19" },
        "astro" => {glyph: "", color: "#FF5D01" },
        "m" => {glyph: "", color: "#ED8012" },
        "rst" => {glyph: "", color: "#74AADA" },
        "cl" => {glyph: "" },
        "njk" => {glyph: "", color: "#53A553" },
        "jinja" => {glyph: "" },
        "bicep" => {glyph: "", color: "#529AB7" },
        "wat" => {glyph: "", color: "#644FEF" },
        "md" => {glyph: "", color: "#519ABA" },
        "mdx" => {glyph: "", color: "#519ABA" },
        "markdown" => {glyph: "", color: "#519ABA" },
        "livemd" => {glyph: "", color: "#519ABA" },
        "make" => {glyph: "", color: "#6D8086" },
        "cmake" => {glyph: "", color: "#064F8C" },
        "nix" => {glyph: "", color: "#7EBAE4" },
        "awk" => {glyph: "" },
        "ll" => {glyph: "", color: "#09627D" },
        "regex" => {glyph: "" },
        "gql" => {glyph: "", color: "#E10098" },
        "typst" => {glyph: "", color: "#239DAD" },
        "json" => {glyph: "", color: "#F7DF1E" },
        "toml" => {glyph: "", color: "#9C4121" },
        "xml" => {glyph: "󰗀", color: "#FF6600" },
        "tex" => {glyph: "", color: "#008080" },
        "todotxt" => {glyph: "", color: "#7CB342" },
        "svg" => {glyph: "󰜡", color: "#FFB300" },
        "png" => {glyph: "", color: "#26A69A" },
        "jpeg" => {glyph: "", color: "#26A69A" },
        "jpg" => {glyph: "", color: "#26A69A" },
        "ico" => {glyph: "", color: "#26A69A" },
        "lock" => {glyph: "", color: "#70797D" },
        "csv" => {glyph: "", color: "#1ABB54" },
        "ipynb" => {glyph: "", color: "#F47724" },
        "ttf" => {glyph: "", color: "#144CB7" },
        "otf" => {glyph: "", color: "#144CB7" },
        "exe" => {glyph: "" },
        "bin" => {glyph: "" },
        "bzl" => {glyph: "", color: "#76D275" },
        "sql" => {glyph: "", color: "#336791" },
        "db" => {glyph: "", color: "#336791" },
        "yaml" => { glyph: "", color: "#CB171E" },
        "yml" => { glyph: "", color: "#CB171E" },
        "conf" => { glyph: "", color: "#6C7086" },
        "ron" => { glyph: "" },
        "hbs" => { glyph: "", color: "#F0772B" },
        "desktop" => { glyph: "", color: "#3DAEE9" },
        "xlsx" => { glyph: "󱎏", color: "#01AC47" },
        "wxs" => { glyph: "" },
        "vim" => { glyph: "", color: "#007F00" },
    }
});

impl Fs {
    /// Returns the icon for a folder/directory if enabled.
    ///
    /// This takes a `bool` that signified if the returned icon should be an open variant or not.
    #[inline]
    pub fn directory(&self, is_open: bool) -> Option<&str> {
        if !self.enabled {
            return None;
        }

        if is_open {
            self.directory_open.as_deref().or(Some("󰝰"))
        } else {
            self.directory.as_deref().or(Some("󰉋"))
        }
    }

    /// Returns an icon that matches an exact name or extension if enabled.
    ///
    /// If there is no match, and is enabled, it will return `None`.
    #[inline]
    pub fn from_name<'a>(&'a self, name: &str) -> Option<&'a Icon> {
        if !self.enabled {
            return None;
        }

        self.mime.get(name).or_else(|| MIMES.get(name))
    }

    /// Returns an icon that matches an exact name or extension if enabled.
    ///
    /// If there is no match, and is enabled, it will return with the default `text` icon.
    #[inline]
    pub fn from_path<'b, 'a: 'b>(&'a self, path: &'b Path) -> Option<&'b Icon> {
        self.__from_path_or_lang(Some(path), None)
    }

    /// Returns an icon that matches an exact name or extension if enabled.
    ///
    /// If there is no match, and is enabled, or if there is `None` passed in, it will
    /// return with the default `text` icon.
    #[inline]
    pub fn from_optional_path<'b, 'a: 'b>(&'a self, path: Option<&'b Path>) -> Option<&'b Icon> {
        self.__from_path_or_lang(path, None)
    }

    /// Returns an icon that matches an exact name, extension, or language, if enabled.
    ///
    /// If there is no match, and is enabled, it will return with the default `text` icon.
    #[inline]
    pub fn from_path_or_lang<'b, 'a: 'b>(
        &'a self,
        path: &'b Path,
        lang: &'b str,
    ) -> Option<&'b Icon> {
        self.__from_path_or_lang(Some(path), Some(lang))
    }

    /// Returns an icon that matches an exact name, extension, or language, if enabled.
    ///
    /// If there is no match, and is enabled, or if there is `None` passed in and there is not language match, it will
    /// return with the default `text` icon.
    #[inline]
    pub fn from_optional_path_or_lang<'b, 'a: 'b>(
        &'a self,
        path: Option<&'b Path>,
        lang: &'b str,
    ) -> Option<&'b Icon> {
        self.__from_path_or_lang(path, Some(lang))
    }

    fn __from_path_or_lang<'b, 'a: 'b>(
        &'a self,
        path: Option<&'b Path>,
        lang: Option<&'b str>,
    ) -> Option<&'b Icon> {
        if !self.enabled {
            return None;
        }

        // Search via some part of the path.
        if let Some(path) = path {
            // Search for fully specified name first so that custom icons,
            // for example for `README.md` or `docker-compose.yaml`, can
            // take precedence over any extension it may have.
            if let Some(Some(name)) = path.file_name().map(|name| name.to_str()) {
                // Search config options first, then built-in.
                if let Some(icon) = self.mime.get(name).or_else(|| MIMES.get(name)) {
                    return Some(icon);
                }
            }

            // Try to search for icons based off of the extension.
            if let Some(Some(ext)) = path.extension().map(|ext| ext.to_str()) {
                // Search config options first, then built-in.
                if let Some(icon) = self.mime.get(ext).or_else(|| MIMES.get(ext)) {
                    return Some(icon);
                }
            }
        }

        // Try to search via lang name.
        if let Some(lang) = lang {
            // Search config options first, then built-in.
            if let Some(icon) = self.mime.get(lang).or_else(|| MIMES.get(lang)) {
                return Some(icon);
            }
        }

        // If icons are enabled but there is no matching found, default to the `text` icon.
        // Check user configured first, then built-in.
        self.mime.get("text").or_else(|| MIMES.get("text"))
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Dap {
    verified: Option<String>,
    unverified: Option<String>,
    play: Option<String>,
}

impl Dap {
    #[inline]
    pub fn verified(&self) -> &str {
        self.verified.as_deref().unwrap_or("●")
    }

    #[inline]
    pub fn unverified(&self) -> &str {
        self.unverified.as_deref().unwrap_or("◯")
    }

    #[inline]
    pub fn play(&self) -> &str {
        self.play.as_deref().unwrap_or("▶")
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Ui {
    workspace: Option<Icon>,
    gutter: Gutter,
    #[serde(rename = "virtual")]
    r#virtual: Virtual,
    statusline: Statusline,
}

impl Ui {
    /// Returns a workspace diagnostic icon.
    ///
    /// If no icon is set in the config, it will return `W` by default.
    #[inline]
    pub fn workspace(&self) -> Icon {
        self.workspace.clone().unwrap_or_else(|| Icon::from("W"))
    }

    #[inline]
    pub fn gutter(&self) -> &Gutter {
        &self.gutter
    }

    #[inline]
    pub fn r#virtual(&self) -> &Virtual {
        &self.r#virtual
    }

    #[inline]
    pub fn statusline(&self) -> &Statusline {
        &self.statusline
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Gutter {
    added: Option<String>,
    modified: Option<String>,
    removed: Option<String>,
}

impl Gutter {
    #[inline]
    pub fn added(&self) -> &str {
        self.added.as_deref().unwrap_or("▍")
    }

    #[inline]
    pub fn modified(&self) -> &str {
        self.modified.as_deref().unwrap_or("▍")
    }

    #[inline]
    pub fn removed(&self) -> &str {
        self.removed.as_deref().unwrap_or("▔")
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Virtual {
    // Whitespace
    space: Option<String>,
    nbsp: Option<String>,
    nnbsp: Option<String>,
    tab: Option<String>,
    newline: Option<String>,
    tabpad: Option<String>,

    // Soft-wrap
    wrap: Option<String>,

    // Indentation guide
    indentation: Option<String>,

    // Ruler
    ruler: Option<String>,
}

impl Virtual {
    #[inline]
    pub fn space(&self) -> &str {
        // Default: U+00B7
        self.space.as_deref().unwrap_or("·")
    }

    #[inline]
    pub fn nbsp(&self) -> &str {
        // Default: U+237D
        self.nbsp.as_deref().unwrap_or("⍽")
    }

    #[inline]
    pub fn nnbsp(&self) -> &str {
        // Default: U+2423
        self.nnbsp.as_deref().unwrap_or("␣")
    }

    #[inline]
    pub fn tab(&self) -> &str {
        // Default: U+2192
        self.tab.as_deref().unwrap_or("→")
    }

    #[inline]
    pub fn newline(&self) -> &str {
        // Default: U+23CE
        self.newline.as_deref().unwrap_or("⏎")
    }

    #[inline]
    pub fn tabpad(&self) -> &str {
        // Default: U+23CE
        self.tabpad.as_deref().unwrap_or(" ")
    }

    #[inline]
    pub fn wrap(&self) -> &str {
        // Default: U+21AA
        self.wrap.as_deref().unwrap_or("↪")
    }

    #[inline]
    pub fn indentation(&self) -> &str {
        // Default: U+254E
        self.indentation.as_deref().unwrap_or("╎")
    }

    #[inline]
    pub fn ruler(&self) -> &str {
        // TODO: Default: U+00A6: ¦
        self.ruler.as_deref().unwrap_or(" ")
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Statusline {
    separator: Option<String>,
}

impl Statusline {
    #[inline]
    pub fn separator(&self) -> &str {
        self.separator.as_deref().unwrap_or("│")
    }
}
