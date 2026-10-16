use cleaner_core::model::{Category, DeleteOutcome, Risk};

pub fn app_title() -> &'static str {
    "Cleaner"
}

pub fn scan_cta() -> &'static str {
    "Iniciar varredura"
}

pub fn clean_cta() -> &'static str {
    "Revisar e limpar"
}


pub fn back_home() -> &'static str {
    "Início"
}

pub fn confirm_title() -> &'static str {
    "Confirmar limpeza"
}

pub fn confirm_body(count: usize, size: &str) -> String {
    format!("Você vai remover {} ({size}). Os arquivos não vão para a Lixeira.", items(count))
}


pub fn scanning() -> &'static str {
    "Procurando caches…"
}

pub fn cleaning() -> &'static str {
    "Limpando"
}

pub fn report_title() -> &'static str {
    "Limpeza concluída"
}

pub fn search_placeholder() -> &'static str {
    "Buscar por nome ou caminho…"
}


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
        Category::Temp => "Temporários",
    }
}

pub fn risk_label(r: Risk) -> &'static str {
    match r {
        Risk::Safe => "Seguro",
        Risk::Review => "Revisar",
        Risk::Blocked => "Bloqueado",
    }
}

pub fn outcome_label(o: DeleteOutcome) -> &'static str {
    match o {
        DeleteOutcome::Removed => "Removido",
        DeleteOutcome::Skipped => "Ignorado",
        DeleteOutcome::Failed => "Falhou",
    }
}




pub fn sort_label(s: cleaner_core::SortKey) -> &'static str {
    use cleaner_core::SortKey::*;
    match s {
        SizeDesc => "Maior primeiro",
        SizeAsc => "Menor primeiro",
        NameAsc => "Nome (A–Z)",
        NameDesc => "Nome (Z–A)",
        Category => "Categoria",
        Risk => "Risco",
    }
}

pub fn items(n: usize) -> String {
    if n == 1 {
        "1 item".to_string()
    } else {
        format!("{n} itens")
    }
}

pub fn risk_hint(r: Risk) -> &'static str {
    match r {
        Risk::Safe => "Pode ser apagado: a ferramenta baixa de novo quando precisar.",
        Risk::Review => "Confira antes: pode conter algo que você ainda usa.",
        Risk::Blocked => "Protegido: o Cleaner não apaga este item.",
    }
}
