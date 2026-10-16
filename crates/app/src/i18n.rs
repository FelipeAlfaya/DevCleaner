//! Interface texts in Portuguese, Spanish and English.
//!
//! Each function returns the text for the current language
//! (`cleaner_core::i18n::lang()`), so the whole UI switches as soon as the
//! language changes. Texts coming from the core (reasons, errors, item
//! names) are translated in `cleaner_core::i18n`.

use cleaner_core::i18n::{lang, pick, Lang};
use cleaner_core::model::{Category, DeleteOutcome, Risk};
use cleaner_core::{ScanProfile, SortKey};

pub fn app_title() -> &'static str {
    "DevCleaner"
}

/// Picks a format string per language and fills it.
macro_rules! tf {
    ($pt:literal, $es:literal, $en:literal $(, $arg:expr)* $(,)?) => {
        match lang() {
            Lang::Pt => format!($pt $(, $arg)*),
            Lang::Es => format!($es $(, $arg)*),
            Lang::En => format!($en $(, $arg)*),
        }
    };
}

pub fn items(n: usize) -> String {
    match (lang(), n) {
        (Lang::Pt, 1) => "1 item".into(),
        (Lang::Pt, _) => format!("{n} itens"),
        (Lang::Es, 1) => "1 elemento".into(),
        (Lang::Es, _) => format!("{n} elementos"),
        (Lang::En, 1) => "1 item".into(),
        (Lang::En, _) => format!("{n} items"),
    }
}

pub fn date_format() -> &'static str {
    pick("%d/%m/%Y", "%d/%m/%Y", "%b %d, %Y")
}

// ------------------------------------------------------------ names

pub fn category_name(c: Category) -> &'static str {
    match c {
        Category::Npm => "npm",
        Category::Yarn => "Yarn",
        Category::Pnpm => "pnpm",
        Category::Bun => "Bun",
        Category::Pip => "pip",
        Category::Uv => "uv",
        Category::Cargo => "Cargo",
        Category::Gradle => "Gradle",
        Category::Maven => "Maven",
        Category::Go => "Go",
        Category::NuGet => "NuGet",
        Category::Composer => "Composer",
        Category::Docker => "Docker",
        Category::Wsl => "WSL",
        Category::Ide => pick("IDEs e apps de IA", "IDEs y apps de IA", "IDEs and AI apps"),
        Category::Temp => pick("Temporários", "Temporales", "Temporary files"),
    }
}

pub fn risk_label(r: Risk) -> &'static str {
    match r {
        Risk::Safe => pick("Seguro", "Seguro", "Safe"),
        Risk::Review => pick("Revisar", "Revisar", "Review"),
        Risk::Blocked => pick("Bloqueado", "Bloqueado", "Blocked"),
    }
}

pub fn risk_hint(r: Risk) -> &'static str {
    match r {
        Risk::Safe => pick(
            "Pode ser apagado: a ferramenta baixa de novo quando precisar.",
            "Se puede borrar: la herramienta lo descarga de nuevo cuando lo necesite.",
            "Safe to delete: the tool downloads it again when needed.",
        ),
        Risk::Review => pick(
            "Confira antes: pode conter algo que só existe aqui.",
            "Revísalo antes: puede contener algo que solo existe aquí.",
            "Check first: it may hold something that only exists here.",
        ),
        Risk::Blocked => pick(
            "Protegido: o DevCleaner não apaga este item.",
            "Protegido: DevCleaner no borra este elemento.",
            "Protected: DevCleaner does not delete this item.",
        ),
    }
}

pub fn outcome_label(o: DeleteOutcome) -> &'static str {
    match o {
        DeleteOutcome::Removed => pick("Removido", "Eliminado", "Removed"),
        DeleteOutcome::Skipped => pick("Ignorado", "Omitido", "Skipped"),
        DeleteOutcome::Failed => pick("Falhou", "Falló", "Failed"),
    }
}

pub fn sort_label(s: SortKey) -> &'static str {
    match s {
        SortKey::SizeDesc => pick("Maior primeiro", "Más grande primero", "Largest first"),
        SortKey::SizeAsc => pick("Menor primeiro", "Más pequeño primero", "Smallest first"),
        SortKey::NameAsc => pick("Nome (A–Z)", "Nombre (A–Z)", "Name (A–Z)"),
        SortKey::NameDesc => pick("Nome (Z–A)", "Nombre (Z–A)", "Name (Z–A)"),
        SortKey::Category => pick("Categoria", "Categoría", "Category"),
        SortKey::Risk => pick("Risco", "Riesgo", "Risk"),
    }
}

// ------------------------------------------------------------ profiles

pub fn profile_name(p: ScanProfile) -> &'static str {
    match p {
        ScanProfile::Full => pick("Completo", "Completo", "Everything"),
        ScanProfile::PackageManagers => pick("Gerenciadores de pacotes", "Gestores de paquetes", "Package managers"),
        ScanProfile::JavaScript => "JavaScript",
        ScanProfile::Python => "Python",
        ScanProfile::Jvm => "Java / Kotlin",
        ScanProfile::RustGo => "Rust / Go",
        ScanProfile::Containers => pick("Docker e WSL", "Docker y WSL", "Docker and WSL"),
        ScanProfile::Editors => pick("IDEs e IA", "IDEs e IA", "IDEs and AI"),
        ScanProfile::Custom => pick("Personalizado", "Personalizado", "Custom"),
    }
}

pub fn profile_hint(p: ScanProfile) -> &'static str {
    match p {
        ScanProfile::Full => pick("Todas as categorias", "Todas las categorías", "Every category"),
        ScanProfile::PackageManagers => pick(
            "Caches de npm, Yarn, pnpm, Bun, pip, uv, Cargo, Gradle, Maven, Go, NuGet e Composer",
            "Cachés de npm, Yarn, pnpm, Bun, pip, uv, Cargo, Gradle, Maven, Go, NuGet y Composer",
            "npm, Yarn, pnpm, Bun, pip, uv, Cargo, Gradle, Maven, Go, NuGet and Composer caches",
        ),
        ScanProfile::JavaScript => "npm, Yarn, pnpm, Bun",
        ScanProfile::Python => "pip, uv",
        ScanProfile::Jvm => "Gradle, Maven",
        ScanProfile::RustGo => "Cargo, Go",
        ScanProfile::Containers => pick(
            "Discos virtuais, imagens, containers e volumes",
            "Discos virtuales, imágenes, contenedores y volúmenes",
            "Virtual disks, images, containers and volumes",
        ),
        ScanProfile::Editors => pick(
            "Cursor, VS Code, Windsurf, Antigravity, Trae, Claude e outros",
            "Cursor, VS Code, Windsurf, Antigravity, Trae, Claude y otros",
            "Cursor, VS Code, Windsurf, Antigravity, Trae, Claude and more",
        ),
        ScanProfile::Custom => pick(
            "Clique nas categorias abaixo para escolher",
            "Haz clic en las categorías de abajo para elegir",
            "Click the categories below to choose",
        ),
    }
}

pub fn scan_profile() -> &'static str {
    pick("Perfil de varredura", "Perfil de análisis", "Scan profile")
}

pub fn profile_tag(name: &str) -> String {
    tf!("Perfil: {name}", "Perfil: {name}", "Profile: {name}")
}

pub fn outside_profile() -> &'static str {
    pick("Fora do perfil", "Fuera del perfil", "Not in profile")
}

pub fn tile_toggle_include() -> &'static str {
    pick("Clique para incluir na varredura", "Haz clic para incluir en el análisis", "Click to include in the scan")
}

pub fn tile_toggle_exclude() -> &'static str {
    pick("Clique para tirar da varredura", "Haz clic para quitar del análisis", "Click to leave out of the scan")
}

pub fn empty_profile() -> &'static str {
    pick(
        "Escolha ao menos uma categoria",
        "Elige al menos una categoría",
        "Pick at least one category",
    )
}

// ------------------------------------------------------------ virtual disks

pub fn vhd_pill() -> &'static str {
    pick("Compactar", "Compactar", "Compact")
}

pub fn disk_size(size: &str) -> String {
    tf!("disco virtual: {size}", "disco virtual: {size}", "virtual disk: {size}")
}

pub fn plus_disks(n: usize) -> String {
    match (lang(), n) {
        (Lang::Pt, 1) => "+ 1 disco virtual para compactar".into(),
        (Lang::Pt, _) => format!("+ {n} discos virtuais para compactar"),
        (Lang::Es, 1) => "+ 1 disco virtual para compactar".into(),
        (Lang::Es, _) => format!("+ {n} discos virtuales para compactar"),
        (Lang::En, 1) => "+ 1 virtual disk to compact".into(),
        (Lang::En, _) => format!("+ {n} virtual disks to compact"),
    }
}

pub fn disks_to_compact() -> &'static str {
    pick("Discos virtuais para compactar", "Discos virtuales para compactar", "Virtual disks to compact")
}

pub fn compact_warning() -> &'static str {
    pick(
        "Para compactar, o DevCleaner desliga o WSL (e com ele o Docker Desktop) e o Windows vai pedir permissão de administrador. Salve o que estiver aberto no WSL antes. Nada é apagado dos discos: só o espaço livre dentro deles volta para o Windows.",
        "Para compactar, DevCleaner apaga WSL (y con él Docker Desktop) y Windows pedirá permiso de administrador. Guarda lo que tengas abierto en WSL antes. No se borra nada de los discos: solo el espacio libre dentro de ellos vuelve a Windows.",
        "To compact, DevCleaner shuts WSL down (and Docker Desktop with it) and Windows will ask for administrator permission. Save anything open in WSL first. Nothing is deleted from the disks: only the free space inside them goes back to Windows.",
    )
}

pub fn compact_disks(n: usize) -> String {
    match (lang(), n) {
        (Lang::Pt, 1) => "Compactar 1 disco".into(),
        (Lang::Pt, _) => format!("Compactar {n} discos"),
        (Lang::Es, 1) => "Compactar 1 disco".into(),
        (Lang::Es, _) => format!("Compactar {n} discos"),
        (Lang::En, 1) => "Compact 1 disk".into(),
        (Lang::En, _) => format!("Compact {n} disks"),
    }
}

pub fn delete_and_compact(size: &str) -> String {
    tf!("Apagar {size} e compactar", "Borrar {size} y compactar", "Delete {size} and compact")
}

pub fn duration(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{h} h {m:02} min")
    } else if m >= 10 || (m > 0 && s == 0) {
        format!("{m} min")
    } else if m > 0 {
        format!("{m} min {s:02} s")
    } else {
        format!("{s} s")
    }
}

/// Rounds an estimate to something a person reads at a glance.
fn rough(secs: u64) -> String {
    if secs < 60 {
        duration(secs.max(10).div_ceil(10) * 10)
    } else {
        let mins = secs.div_ceil(60);
        let mins = if mins > 20 { mins.div_ceil(5) * 5 } else { mins };
        duration(mins * 60)
    }
}

pub fn compact_estimate(fast: u64, slow: u64) -> String {
    let (a, b) = (rough(fast), rough(slow));
    tf!("Tempo estimado: de {a} a {b}", "Tiempo estimado: de {a} a {b}", "Estimated time: {a} to {b}")
}

pub fn compact_slow_note() -> &'static str {
    pick(
        "A compactação lê o disco inteiro, então pode demorar bastante: depende do tamanho do disco e de ele estar num SSD ou num HD. Não feche o DevCleaner nem desligue o computador até terminar.",
        "La compactación lee el disco entero, así que puede tardar bastante: depende del tamaño del disco y de si está en un SSD o en un HDD. No cierres DevCleaner ni apagues el equipo hasta que termine.",
        "Compacting reads the whole disk, so it can take a long time depending on the disk size and on whether it sits on an SSD or a hard drive. Do not close DevCleaner or turn off the computer until it finishes.",
    )
}

pub fn compacting_title() -> &'static str {
    pick("Compactando discos virtuais", "Compactando discos virtuales", "Compacting virtual disks")
}

pub fn compacting_disk(i: usize, n: usize) -> String {
    tf!("Disco {i} de {n}", "Disco {i} de {n}", "Disk {i} of {n}")
}

pub fn elapsed(d: &str) -> String {
    tf!("Decorrido: {d}", "Transcurrido: {d}", "Elapsed: {d}")
}

pub fn waiting_admin() -> &'static str {
    pick(
        "Aguardando a permissão de administrador e o desligamento do WSL…",
        "Esperando el permiso de administrador y el apagado de WSL…",
        "Waiting for administrator permission and for WSL to shut down…",
    )
}

pub fn past_estimate() -> &'static str {
    pick(
        "Está levando mais que o previsto. Isso é normal em discos grandes ou em HD; a compactação continua.",
        "Está tardando más de lo previsto. Es normal en discos grandes o en HDD; la compactación continúa.",
        "This is taking longer than expected. That is normal for large disks or hard drives; compaction is still running.",
    )
}

// ------------------------------------------------------------ top bar

pub fn nav_home() -> &'static str {
    pick("Início", "Inicio", "Home")
}

pub fn nav_results() -> &'static str {
    pick("Resultados", "Resultados", "Results")
}

pub fn theme_toggle(dark: bool) -> &'static str {
    if dark {
        pick("Tema claro", "Tema claro", "Light theme")
    } else {
        pick("Tema escuro", "Tema oscuro", "Dark theme")
    }
}

pub fn scanning_short(done: usize, total: usize) -> String {
    tf!("Varrendo {done}/{total}", "Analizando {done}/{total}", "Scanning {done}/{total}")
}

pub fn language() -> &'static str {
    pick("Idioma", "Idioma", "Language")
}

// ------------------------------------------------------------ home

pub fn hero_title() -> &'static str {
    pick(
        "Seus caches de desenvolvimento crescem em silêncio.",
        "Tus cachés de desarrollo crecen en silencio.",
        "Your development caches grow quietly.",
    )
}

pub fn hero_body() -> &'static str {
    pick(
        "O DevCleaner procura caches de npm, Cargo, Gradle e outros gerenciadores de pacotes, do Docker, do WSL e de editores como Cursor e VS Code, mostra quanto cada um ocupa e só apaga o que você aprovar.",
        "DevCleaner busca las cachés de npm, Cargo, Gradle y otros gestores de paquetes, de Docker, de WSL y de editores como Cursor y VS Code, muestra cuánto ocupa cada una y solo borra lo que tú apruebes.",
        "DevCleaner finds the caches left by npm, Cargo, Gradle and other package managers, Docker, WSL and editors such as Cursor and VS Code, shows how much each one takes and only deletes what you approve.",
    )
}

pub fn scan_cta() -> &'static str {
    pick("Iniciar varredura", "Iniciar análisis", "Start scan")
}

pub fn view_scan_in_progress() -> &'static str {
    pick("Ver varredura em andamento", "Ver análisis en curso", "View scan in progress")
}

pub fn view_results() -> &'static str {
    pick("Ver resultados", "Ver resultados", "View results")
}

pub fn scan_again() -> &'static str {
    pick("Varrer de novo", "Analizar de nuevo", "Scan again")
}

pub fn stop_scan() -> &'static str {
    pick("Parar varredura", "Detener análisis", "Stop scan")
}

pub fn found_summary(n: usize) -> String {
    tf!(
        "em caches encontrados, {}",
        "en cachés encontradas, {}",
        "of caches found, {}",
        items(n)
    )
}

pub fn where_we_look() -> &'static str {
    pick("Onde o DevCleaner procura", "Dónde busca DevCleaner", "Where DevCleaner looks")
}

pub fn tile_searching() -> &'static str {
    pick("Procurando…", "Buscando…", "Searching…")
}

pub fn tile_not_checked() -> &'static str {
    pick("Ainda não verificado", "Aún no revisado", "Not checked yet")
}

pub fn tile_nothing() -> &'static str {
    pick("Nada encontrado", "Nada encontrado", "Nothing found")
}

pub fn tile_hover() -> &'static str {
    pick("Ver só esta categoria", "Ver solo esta categoría", "Show only this category")
}

pub fn preferences() -> &'static str {
    pick("Preferências", "Preferencias", "Preferences")
}

pub fn temp_age_title() -> &'static str {
    pick("Idade mínima dos temporários", "Antigüedad mínima de los temporales", "Minimum age of temporary files")
}

pub fn temp_age_body() -> &'static str {
    pick(
        "Arquivos temporários mais novos que isso ficam de fora da varredura.",
        "Los archivos temporales más recientes que esto quedan fuera del análisis.",
        "Temporary files newer than this are left out of the scan.",
    )
}

pub fn days_suffix() -> &'static str {
    pick(" dias", " días", " days")
}

pub fn language_body() -> &'static str {
    pick(
        "Idioma da interface e das mensagens.",
        "Idioma de la interfaz y de los mensajes.",
        "Language of the interface and messages.",
    )
}

// ------------------------------------------------------------ results

pub fn will_delete() -> &'static str {
    pick("Será excluído", "Se eliminará", "Will be deleted")
}

pub fn marked(n: usize) -> String {
    tf!("{} marcados", "{} marcados", "{} selected", items(n))
}

pub fn share_of_found(pct: f64, total: &str) -> String {
    tf!(
        "{pct:.0}% dos {total} encontrados",
        "{pct:.0}% de los {total} encontrados",
        "{pct:.0}% of the {total} found"
    )
}

pub fn clean_cta() -> &'static str {
    pick("Revisar e limpar", "Revisar y limpiar", "Review and clean")
}

pub fn wait_for_scan() -> &'static str {
    pick("Aguarde a varredura terminar", "Espera a que termine el análisis", "Wait for the scan to finish")
}

pub fn select_something() -> &'static str {
    pick("Marque ao menos um item", "Marca al menos un elemento", "Select at least one item")
}

pub fn results_title() -> &'static str {
    pick("Resultados", "Resultados", "Results")
}

pub fn scanning_progress(done: usize, total: usize) -> String {
    tf!(
        "Procurando caches… {done} de {total} categorias",
        "Buscando cachés… {done} de {total} categorías",
        "Looking for caches… {done} of {total} categories"
    )
}

pub fn total_in(size: &str, n: usize) -> String {
    tf!("{size} em {}", "{size} en {}", "{size} in {}", items(n))
}

pub fn more_categories(n: usize) -> String {
    tf!("+{n} categorias", "+{n} categorías", "+{n} categories")
}

pub fn search_placeholder() -> &'static str {
    pick("Buscar por nome ou caminho…", "Buscar por nombre o ruta…", "Search by name or path…")
}

pub fn sort_by() -> &'static str {
    pick("Ordenar", "Ordenar", "Sort")
}

pub fn only_selected() -> &'static str {
    pick("Só marcados", "Solo marcados", "Selected only")
}

pub fn select_visible() -> &'static str {
    pick("Marcar visíveis", "Marcar visibles", "Select visible")
}

pub fn unselect_visible() -> &'static str {
    pick("Desmarcar visíveis", "Desmarcar visibles", "Unselect visible")
}

pub fn all_categories() -> &'static str {
    pick("Todas", "Todas", "All")
}

pub fn chip_toggle_hint() -> &'static str {
    pick(
        "Clique para incluir ou remover do filtro",
        "Haz clic para añadir o quitar del filtro",
        "Click to add to or remove from the filter",
    )
}

pub fn clear_filters() -> &'static str {
    pick("Limpar filtros", "Quitar filtros", "Clear filters")
}

pub fn nothing_to_clean_in(list: &str) -> String {
    tf!("Sem nada para limpar: {list}", "Nada que limpiar en: {list}", "Nothing to clean in: {list}")
}

pub fn empty_scanning() -> &'static str {
    pick(
        "Procurando caches. Os itens aparecem aqui conforme são encontrados.",
        "Buscando cachés. Los elementos aparecen aquí a medida que se encuentran.",
        "Looking for caches. Items show up here as they are found.",
    )
}

pub fn empty_title() -> &'static str {
    pick("Nada para limpar", "Nada que limpiar", "Nothing to clean")
}

pub fn empty_body() -> &'static str {
    pick(
        "Nenhum cache encontrado. Seu disco agradece.",
        "No se encontró ninguna caché. Tu disco lo agradece.",
        "No caches found. Your disk thanks you.",
    )
}

pub fn empty_filtered() -> &'static str {
    pick("Nenhum item com esses filtros", "Ningún elemento con estos filtros", "No items match these filters")
}

pub fn modified_on(date: &str) -> String {
    tf!("Modificado em {date}", "Modificado el {date}", "Modified on {date}")
}

pub fn strata_hover(found: &str, selected: &str) -> String {
    tf!(
        "{found} encontrados, {selected} marcados",
        "{found} encontrados, {selected} marcados",
        "{found} found, {selected} selected"
    )
}

pub fn click_to_filter() -> &'static str {
    pick("Clique para filtrar", "Haz clic para filtrar", "Click to filter")
}

// ------------------------------------------------------------ confirm

pub fn confirm_title() -> &'static str {
    pick("Confirmar limpeza", "Confirmar limpieza", "Confirm cleanup")
}

pub fn confirm_body(count: usize, size: &str) -> String {
    tf!(
        "Você vai remover {} ({size}). Os arquivos não vão para a Lixeira.",
        "Vas a eliminar {} ({size}). Los archivos no van a la Papelera.",
        "You are about to remove {} ({size}). Files do not go to the Recycle Bin.",
        items(count)
    )
}

pub fn space_to_free() -> &'static str {
    pick("Espaço a liberar", "Espacio a liberar", "Space to free")
}

pub fn review_warning() -> &'static str {
    pick(
        "Há itens marcados como Revisar. Eles podem conter algo que só existe aqui.",
        "Hay elementos marcados como Revisar. Pueden contener algo que solo existe aquí.",
        "Some selected items are marked Review. They may hold something that only exists here.",
    )
}

pub fn see_items(count: usize) -> String {
    tf!("Ver os {}", "Ver los {}", "See the {}", items(count))
}

pub fn back_and_review() -> &'static str {
    pick("Voltar e revisar", "Volver y revisar", "Go back and review")
}

pub fn delete_size(size: &str) -> String {
    tf!("Apagar {size}", "Borrar {size}", "Delete {size}")
}

// ------------------------------------------------------------ cleaning & report

pub fn cleaning() -> &'static str {
    pick("Limpando", "Limpiando", "Cleaning")
}

pub fn done_of(done: usize, total: usize) -> String {
    tf!("{done} de {total}", "{done} de {total}", "{done} of {total}")
}

pub fn report_title() -> &'static str {
    pick("Limpeza concluída", "Limpieza completada", "Cleanup finished")
}

pub fn freed_on_disk() -> &'static str {
    pick("liberados no seu disco", "liberados en tu disco", "freed on your disk")
}

pub fn stat_removed() -> &'static str {
    pick("removidos", "eliminados", "removed")
}

pub fn stat_skipped() -> &'static str {
    pick("ignorados", "omitidos", "skipped")
}

pub fn stat_failed() -> &'static str {
    pick("com falha", "con error", "failed")
}

pub fn back_home() -> &'static str {
    pick("Início", "Inicio", "Home")
}

pub fn new_scan() -> &'static str {
    pick("Nova varredura", "Nuevo análisis", "New scan")
}
