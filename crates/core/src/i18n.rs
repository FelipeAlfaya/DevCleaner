//! Languages supported by DevCleaner and the texts produced by the core.
//!
//! The core never stores translated strings. Scanners and the delete engine
//! emit [`Text`] values (a message key, optionally with an argument, or raw
//! data such as a file name) that are rendered in the current language when
//! displayed. Switching language therefore updates everything on screen
//! without scanning again.

use std::fmt;
use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lang {
    #[serde(rename = "pt")]
    Pt,
    #[serde(rename = "es")]
    Es,
    #[serde(rename = "en")]
    En,
}

impl Lang {
    pub fn all() -> &'static [Lang] {
        &[Lang::Pt, Lang::Es, Lang::En]
    }

    /// Name of the language written in that language.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::Pt => "Português",
            Lang::Es => "Español",
            Lang::En => "English",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::Pt => "pt",
            Lang::Es => "es",
            Lang::En => "en",
        }
    }

    /// Maps a BCP-47 tag such as `pt-BR`, `es_MX` or `en-US` to a supported
    /// language. Anything else falls back to English.
    pub fn from_tag(tag: &str) -> Lang {
        let primary = tag
            .split(['-', '_', '.'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        match primary.as_str() {
            "pt" => Lang::Pt,
            "es" => Lang::Es,
            _ => Lang::En,
        }
    }

    /// Language of the operating system, or English when it is not supported.
    pub fn detect() -> Lang {
        sys_locale::get_locale()
            .map(|t| Lang::from_tag(&t))
            .unwrap_or(Lang::En)
    }

    fn index(self) -> u8 {
        match self {
            Lang::Pt => 0,
            Lang::Es => 1,
            Lang::En => 2,
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(2);

pub fn set_lang(lang: Lang) {
    CURRENT.store(lang.index(), Ordering::Relaxed);
}

pub fn lang() -> Lang {
    match CURRENT.load(Ordering::Relaxed) {
        0 => Lang::Pt,
        1 => Lang::Es,
        _ => Lang::En,
    }
}

/// Picks the variant for the current language.
pub fn pick(pt: &'static str, es: &'static str, en: &'static str) -> &'static str {
    match lang() {
        Lang::Pt => pt,
        Lang::Es => es,
        Lang::En => en,
    }
}

/// A user-facing text coming from the core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Text {
    /// Data shown as is (file names, paths, ids, OS error messages).
    Raw(String),
    Msg(Msg),
    /// A message whose translation contains `{}`, replaced by the argument.
    MsgArg(Msg, String),
}

impl Text {
    pub fn with(msg: Msg, arg: impl Into<String>) -> Self {
        Text::MsgArg(msg, arg.into())
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Text::Raw(s) => f.write_str(s),
            Text::Msg(m) => f.write_str(m.text()),
            Text::MsgArg(m, a) => f.write_str(&m.text().replace("{}", a)),
        }
    }
}

impl From<String> for Text {
    fn from(s: String) -> Self {
        Text::Raw(s)
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Text::Raw(s.to_string())
    }
}

impl From<Msg> for Text {
    fn from(m: Msg) -> Self {
        Text::Msg(m)
    }
}

/// Every fixed text the core can produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Msg {
    // npm
    NpmLabel,
    NpmReason,
    NpmNone,
    // pnpm
    PnpmLabel,
    PnpmReason,
    PnpmNone,
    // Yarn
    YarnClassic,
    YarnBerry,
    YarnNone,
    // Bun
    BunEntry,
    BunNone,
    // pip
    PipEntry,
    PipNone,
    // uv
    UvEntry,
    UvNone,
    // Cargo
    CargoSubtree,
    CargoEntry,
    CargoNone,
    // Gradle
    GradleGroup,
    GradleArtifact,
    GradleNone,
    // Maven
    MavenGroup,
    MavenCoord,
    MavenNone,
    // Go
    GoBuildLabel,
    GoBuildReason,
    GoModLabel,
    GoModReason,
    GoNone,
    // NuGet
    NugetPackage,
    NugetNone,
    // Composer
    ComposerEntry,
    ComposerNone,
    // Docker
    DockerUnavailable,
    DockerImage,
    DockerContainer,
    DockerVolumeLabel,
    DockerVolumeReason,
    DockerBuildCacheLabel,
    DockerBuildCacheReason,
    DockerDiskLabel,
    DockerDiskReason,
    DockerUnusedImage,
    // WSL
    WslWindowsOnly,
    WslNone,
    WslDiskLabel,
    WslDiskReason,
    WslCache,
    // Virtual disk compaction
    VhdCompacting,
    VhdCompactFailed,
    VhdNothingFreed,
    VhdWindowsOnly,
    // IDEs and AI apps
    IdeCache,
    IdeUpdater,
    IdeWorkspaceStorage,
    IdeWorkspaceEntry,
    IdeCursorIndex,
    IdeClaudeCodeProjects,
    IdeNone,
    // Temporary files
    TempReason,
    TempContent,
    TempNone,
    // Safety
    SafetyInvalidPath,
    SafetyOutsideJail,
    SafetyProtected,
    SafetySymlink,
    // Delete
    DeleteAlreadyRun,
    DeleteBlocked,
    DeleteToolFailed,
    DeleteDockerFailed,
}

impl Msg {
    pub fn text(self) -> &'static str {
        use Msg::*;
        match self {
            NpmLabel => pick("Cache do npm (_cacache)", "Caché de npm (_cacache)", "npm cache (_cacache)"),
            NpmReason => pick(
                "Limpo com npm cache clean --force (não é seguro apagar arquivos isolados)",
                "Se limpia con npm cache clean --force (no es seguro borrar archivos sueltos)",
                "Cleaned with npm cache clean --force (deleting single files is not safe)",
            ),
            NpmNone => pick("Cache do npm não encontrado", "No se encontró la caché de npm", "npm cache not found"),

            PnpmLabel => pick("Store do pnpm (prune)", "Store de pnpm (prune)", "pnpm store (prune)"),
            PnpmReason => pick(
                "Remove pacotes que nenhum projeto usa, com pnpm store prune",
                "Elimina paquetes que ningún proyecto usa, con pnpm store prune",
                "Removes packages no project uses, with pnpm store prune",
            ),
            PnpmNone => pick("Store do pnpm não encontrado", "No se encontró el store de pnpm", "pnpm store not found"),

            YarnClassic => pick("Pacote em cache do Yarn Classic", "Paquete en caché de Yarn Classic", "Cached Yarn Classic package"),
            YarnBerry => pick("Pacote zip do Yarn Berry", "Paquete zip de Yarn Berry", "Yarn Berry zip package"),
            YarnNone => pick("Cache do Yarn não encontrado", "No se encontró la caché de Yarn", "Yarn cache not found"),

            BunEntry => pick("Item do cache do Bun", "Elemento de la caché de Bun", "Bun cache entry"),
            BunNone => pick("Cache do Bun não encontrado", "No se encontró la caché de Bun", "Bun cache not found"),

            PipEntry => pick("Wheel/sdist no cache do pip", "Wheel/sdist en la caché de pip", "Wheel/sdist in the pip cache"),
            PipNone => pick("Cache do pip não encontrado", "No se encontró la caché de pip", "pip cache not found"),

            UvEntry => pick("Item do cache do uv", "Elemento de la caché de uv", "uv cache entry"),
            UvNone => pick("Cache do uv não encontrado", "No se encontró la caché de uv", "uv cache not found"),

            CargoSubtree => pick("Parte do cache do Cargo", "Parte de la caché de Cargo", "Part of the Cargo cache"),
            CargoEntry => pick("Item do Cargo", "Elemento de Cargo", "Cargo entry"),
            CargoNone => pick(
                "CARGO_HOME não encontrado ou vazio",
                "CARGO_HOME no encontrado o vacío",
                "CARGO_HOME not found or empty",
            ),

            GradleGroup => pick("Grupo do cache do Gradle", "Grupo de la caché de Gradle", "Gradle cache group"),
            GradleArtifact => pick("Artefato do Gradle", "Artefacto de Gradle", "Gradle artifact"),
            GradleNone => pick("Cache do Gradle não encontrado", "No se encontró la caché de Gradle", "Gradle cache not found"),

            MavenGroup => pick(
                "Grupo do Maven (~/.m2/repository). Pode incluir artefatos instalados localmente com mvn install",
                "Grupo de Maven (~/.m2/repository). Puede incluir artefactos instalados localmente con mvn install",
                "Maven group (~/.m2/repository). May include artifacts installed locally with mvn install",
            ),
            MavenCoord => pick("Coordenada do Maven", "Coordenada de Maven", "Maven coordinate"),
            MavenNone => pick(
                "Repositório local do Maven não encontrado",
                "No se encontró el repositorio local de Maven",
                "Local Maven repository not found",
            ),

            GoBuildLabel => pick("Cache de build do Go", "Caché de compilación de Go", "Go build cache"),
            GoBuildReason => pick("Limpo com go clean -cache", "Se limpia con go clean -cache", "Cleaned with go clean -cache"),
            GoModLabel => pick("Módulos Go (pkg/mod)", "Módulos de Go (pkg/mod)", "Go modules (pkg/mod)"),
            GoModReason => pick(
                "Limpo com go clean -modcache. Os módulos são baixados de novo no próximo build",
                "Se limpia con go clean -modcache. Los módulos se descargan de nuevo en la próxima compilación",
                "Cleaned with go clean -modcache. Modules are downloaded again on the next build",
            ),
            GoNone => pick("Cache do Go não encontrado", "No se encontró la caché de Go", "Go cache not found"),

            NugetPackage => pick("Pacote global do NuGet", "Paquete global de NuGet", "Global NuGet package"),
            NugetNone => pick("Cache do NuGet não encontrado", "No se encontró la caché de NuGet", "NuGet cache not found"),

            ComposerEntry => pick("Item do cache do Composer", "Elemento de la caché de Composer", "Composer cache entry"),
            ComposerNone => pick(
                "Cache do Composer não encontrado",
                "No se encontró la caché de Composer",
                "Composer cache not found",
            ),

            DockerUnavailable => pick(
                "Docker CLI indisponível ou daemon parado",
                "Docker CLI no disponible o daemon detenido",
                "Docker CLI unavailable or daemon stopped",
            ),
            DockerImage => pick(
                "Imagem Docker sem tag, removida com docker rmi",
                "Imagen de Docker sin etiqueta, se elimina con docker rmi",
                "Untagged Docker image, removed with docker rmi",
            ),
            DockerContainer => pick(
                "Container Docker, removido com docker rm. Tudo o que foi gravado dentro dele fora de volumes é perdido",
                "Contenedor de Docker, se elimina con docker rm. Se pierde todo lo escrito dentro de él fuera de volúmenes",
                "Docker container, removed with docker rm. Anything written inside it outside volumes is lost",
            ),
            DockerVolumeLabel => pick("Volume órfão {}", "Volumen huérfano {}", "Orphan volume {}"),
            DockerVolumeReason => pick(
                "Volume Docker que nenhum container usa. Pode conter dados de banco",
                "Volumen de Docker que ningún contenedor usa. Puede contener datos de bases de datos",
                "Docker volume no container uses. It may hold database data",
            ),
            DockerBuildCacheLabel => pick("Cache de build {}", "Caché de compilación {}", "Build cache {}"),
            DockerBuildCacheReason => pick(
                "Item do cache de build do Docker",
                "Elemento de la caché de compilación de Docker",
                "Docker build cache entry",
            ),

            DockerDiskLabel => pick(
                "Disco virtual do Docker Desktop ({})",
                "Disco virtual de Docker Desktop ({})",
                "Docker Desktop virtual disk ({})",
            ),
            DockerDiskReason => pick(
                "Onde o Docker Desktop guarda imagens, containers e volumes. O arquivo cresce mas não diminui sozinho, mesmo depois de apagar imagens. Marcar compacta o disco: nada é apagado, mas o WSL e o Docker são desligados e o Windows pede permissão de administrador",
                "Donde Docker Desktop guarda imágenes, contenedores y volúmenes. El archivo crece pero no se reduce solo, ni siquiera tras borrar imágenes. Marcarlo compacta el disco: no se borra nada, pero WSL y Docker se apagan y Windows pide permiso de administrador",
                "Where Docker Desktop keeps images, containers and volumes. The file grows but never shrinks on its own, even after deleting images. Selecting it compacts the disk: nothing is deleted, but WSL and Docker are shut down and Windows asks for administrator permission",
            ),
            DockerUnusedImage => pick(
                "Imagem que nenhum container usa. Volta com docker pull quando precisar",
                "Imagen que ningún contenedor usa. Vuelve con docker pull cuando la necesites",
                "Image no container uses. Comes back with docker pull when needed",
            ),
            WslWindowsOnly => pick("Disponível só no Windows", "Disponible solo en Windows", "Windows only"),
            WslNone => pick("Nenhuma distro WSL encontrada", "No se encontró ninguna distro de WSL", "No WSL distro found"),
            WslDiskLabel => pick("Disco virtual da distro {}", "Disco virtual de la distro {}", "Virtual disk of the {} distro"),
            WslDiskReason => pick(
                "O arquivo da distro cresce mas não diminui sozinho. Marcar compacta o disco: nada é apagado, mas o WSL é desligado e o Windows pede permissão de administrador",
                "El archivo de la distro crece pero no se reduce solo. Marcarlo compacta el disco: no se borra nada, pero WSL se apaga y Windows pide permiso de administrador",
                "The distro file grows but never shrinks on its own. Selecting it compacts the disk: nothing is deleted, but WSL is shut down and Windows asks for administrator permission",
            ),
            WslCache => pick(
                "Cache dentro da distro. Depois de apagar, compacte o disco da distro para o Windows recuperar o espaço",
                "Caché dentro de la distro. Después de borrar, compacta el disco de la distro para que Windows recupere el espacio",
                "Cache inside the distro. After deleting, compact the distro disk so Windows gets the space back",
            ),
            VhdCompacting => pick("Compactando discos virtuais…", "Compactando discos virtuales…", "Compacting virtual disks…"),
            VhdCompactFailed => pick(
                "A compactação falhou ou a permissão de administrador foi negada",
                "La compactación falló o se denegó el permiso de administrador",
                "Compaction failed or administrator permission was denied",
            ),
            VhdNothingFreed => pick(
                "Nada a recuperar: o disco já estava compacto",
                "Nada que recuperar: el disco ya estaba compacto",
                "Nothing to reclaim: the disk was already compact",
            ),
            VhdWindowsOnly => pick("Compactação disponível só no Windows", "Compactación disponible solo en Windows", "Compaction is Windows only"),
            IdeCache => pick(
                "Cache que o app recria sozinho. Feche o app antes de limpar",
                "Caché que la app recrea sola. Cierra la app antes de limpiar",
                "Cache the app rebuilds on its own. Close the app before cleaning",
            ),
            IdeUpdater => pick(
                "Instaladores de atualização já baixados",
                "Instaladores de actualización ya descargados",
                "Update installers already downloaded",
            ),
            IdeWorkspaceStorage => pick(
                "Estado salvo de cada projeto aberto no editor. No Cursor inclui o histórico de chat da IA daquele projeto",
                "Estado guardado de cada proyecto abierto en el editor. En Cursor incluye el historial de chat de la IA de ese proyecto",
                "Saved state of each project opened in the editor. In Cursor it includes that project's AI chat history",
            ),
            IdeWorkspaceEntry => pick(
                "Estado salvo deste projeto no editor",
                "Estado guardado de este proyecto en el editor",
                "Saved editor state of this project",
            ),
            IdeCursorIndex => pick(
                "Índice local do código usado pela IA do Cursor. É refeito ao abrir o projeto de novo",
                "Índice local del código que usa la IA de Cursor. Se rehace al abrir el proyecto de nuevo",
                "Local code index used by Cursor's AI. Rebuilt when the project is opened again",
            ),
            IdeClaudeCodeProjects => pick(
                "Histórico das conversas do Claude Code, uma pasta por projeto. Apagar impede retomar essas conversas",
                "Historial de las conversaciones de Claude Code, una carpeta por proyecto. Borrarlo impide retomar esas conversaciones",
                "Claude Code conversation history, one folder per project. Deleting it means those conversations can no longer be resumed",
            ),
            IdeNone => pick(
                "Nenhum cache de editor ou app de IA encontrado",
                "No se encontró ninguna caché de editores o apps de IA",
                "No editor or AI app cache found",
            ),
            TempReason => pick(
                "Temporário com mais de {} dias",
                "Temporal con más de {} días",
                "Temporary file older than {} days",
            ),
            TempContent => pick("Conteúdo temporário", "Contenido temporal", "Temporary content"),
            TempNone => pick(
                "Nenhum temporário antigo o bastante",
                "Ningún temporal es lo bastante antiguo",
                "No temporary file is old enough",
            ),

            SafetyInvalidPath => pick("Caminho inválido", "Ruta no válida", "Invalid path"),
            SafetyOutsideJail => pick(
                "Caminho fora da pasta do cache",
                "Ruta fuera de la carpeta de la caché",
                "Path outside the cache folder",
            ),
            SafetyProtected => pick("Caminho protegido do sistema", "Ruta protegida del sistema", "Protected system path"),
            SafetySymlink => pick(
                "Links simbólicos e junctions não são permitidos",
                "No se permiten enlaces simbólicos ni junctions",
                "Symlinks and junctions are not allowed",
            ),

            DeleteAlreadyRun => pick("Ação já executada", "Acción ya ejecutada", "Action already run"),
            DeleteBlocked => pick("Item bloqueado", "Elemento bloqueado", "Item is blocked"),
            DeleteToolFailed => pick(
                "O comando da ferramenta falhou ou ela não está no PATH",
                "El comando de la herramienta falló o no está en el PATH",
                "The tool command failed or the tool is not in PATH",
            ),
            DeleteDockerFailed => pick("O comando docker falhou", "El comando docker falló", "The docker command failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_map_to_supported_languages() {
        assert_eq!(Lang::from_tag("pt-BR"), Lang::Pt);
        assert_eq!(Lang::from_tag("pt_PT.UTF-8"), Lang::Pt);
        assert_eq!(Lang::from_tag("es-MX"), Lang::Es);
        assert_eq!(Lang::from_tag("en-US"), Lang::En);
        assert_eq!(Lang::from_tag("fr-FR"), Lang::En);
        assert_eq!(Lang::from_tag(""), Lang::En);
    }

    #[test]
    fn text_with_argument() {
        set_lang(Lang::En);
        assert_eq!(Text::with(Msg::TempReason, "7").to_string(), "Temporary file older than 7 days");
        assert_eq!(Text::from("raw").to_string(), "raw");
    }
}
