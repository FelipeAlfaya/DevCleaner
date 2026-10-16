//! Interface texts in Portuguese, Spanish and English.
//!
//! Each function returns the text for the current language
//! (`cleaner_core::i18n::lang()`), so the whole UI switches as soon as the
//! language changes. Texts coming from the core (reasons, errors, item
//! names) are translated in `cleaner_core::i18n`.

use cleaner_core::i18n::{lang, pick, Lang};
use cleaner_core::model::{Category, DeleteOutcome, Risk};
use cleaner_core::SortKey;

pub fn app_title() -> &'static str {
    "Cleaner"
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
            "Protegido: o Cleaner não apaga este item.",
            "Protegido: Cleaner no borra este elemento.",
            "Protected: Cleaner does not delete this item.",
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
        "O Cleaner procura caches de npm, Cargo, Gradle, Docker e outros 10 gerenciadores, mostra quanto cada um ocupa e só apaga o que você aprovar.",
        "Cleaner busca las cachés de npm, Cargo, Gradle, Docker y otros 10 gestores, muestra cuánto ocupa cada una y solo borra lo que tú apruebes.",
        "Cleaner finds the caches left by npm, Cargo, Gradle, Docker and 10 other tools, shows how much each one takes and only deletes what you approve.",
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
    pick("Onde o Cleaner procura", "Dónde busca Cleaner", "Where Cleaner looks")
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
