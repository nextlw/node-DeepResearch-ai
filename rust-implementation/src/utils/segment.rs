// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SEGMENT - Chunking de Texto
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Baseado em src/tools/segment.ts do TypeScript.
// Divide texto em chunks para processamento de referências semânticas.
//
// Estratégias de chunking:
// - Newline: Split por quebras de linha
// - Punctuation: Split por pontuação (. ! ? 。 ！ ？)
// - Characters: Split por N caracteres
// - Regex: Split por padrão regex customizado
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use regex::Regex;

/// Tipo de chunking a ser aplicado
#[derive(Debug, Clone)]
pub enum ChunkType {
    /// Split por quebras de linha (\n)
    Newline,
    /// Split por pontuação comum (. ! ? 。 ！ ？)
    Punctuation,
    /// Split por N caracteres
    Characters(usize),
    /// Split por padrão regex customizado
    Regex(String),
}

impl Default for ChunkType {
    fn default() -> Self {
        Self::Newline
    }
}

/// Opções de configuração para chunking
#[derive(Debug, Clone)]
pub struct ChunkOptions {
    /// Tipo de chunking a aplicar
    pub chunk_type: ChunkType,
    /// Tamanho mínimo do chunk em caracteres (default: 80)
    pub min_chunk_length: usize,
}

impl Default for ChunkOptions {
    fn default() -> Self {
        Self {
            chunk_type: ChunkType::Newline,
            min_chunk_length: 80,
        }
    }
}

impl ChunkOptions {
    /// Cria opções com chunking por newline
    pub fn newline() -> Self {
        Self::default()
    }

    /// Cria opções com chunking por pontuação
    pub fn punctuation() -> Self {
        Self {
            chunk_type: ChunkType::Punctuation,
            min_chunk_length: 80,
        }
    }

    /// Cria opções com chunking por N caracteres
    pub fn characters(size: usize) -> Self {
        Self {
            chunk_type: ChunkType::Characters(size),
            min_chunk_length: 80,
        }
    }

    /// Cria opções com chunking por regex
    pub fn regex(pattern: &str) -> Self {
        Self {
            chunk_type: ChunkType::Regex(pattern.to_string()),
            min_chunk_length: 80,
        }
    }

    /// Define o tamanho mínimo do chunk
    pub fn with_min_length(mut self, min_length: usize) -> Self {
        self.min_chunk_length = min_length;
        self
    }
}

/// Resultado do chunking de texto
#[derive(Debug, Clone)]
pub struct ChunkResult {
    /// Lista de chunks de texto
    pub chunks: Vec<String>,
    /// Posições (start, end) de cada chunk no texto original
    pub positions: Vec<(usize, usize)>,
}

impl ChunkResult {
    /// Retorna true se não há chunks
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Retorna o número de chunks
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Itera sobre chunks com suas posições
    pub fn iter(&self) -> impl Iterator<Item = (&String, &(usize, usize))> {
        self.chunks.iter().zip(self.positions.iter())
    }
}

/// Divide texto em chunks baseado nas opções fornecidas.
///
/// # Argumentos
/// * `text` - Texto a ser dividido
/// * `options` - Opções de chunking (tipo, tamanho mínimo)
///
/// # Retorna
/// `ChunkResult` contendo os chunks e suas posições no texto original
///
/// # Exemplo
/// ```rust
/// use deep_research::utils::segment::{chunk_text, ChunkOptions};
///
/// let text = "Primeira linha.\nSegunda linha.\nTerceira linha.";
/// let result = chunk_text(text, &ChunkOptions::newline().with_min_length(10));
///
/// assert_eq!(result.chunks.len(), 3);
/// ```
pub fn chunk_text(text: &str, options: &ChunkOptions) -> ChunkResult {
    let raw_chunks = split_by_type(text, &options.chunk_type);

    // Filtrar chunks vazios e menores que o mínimo, calculando posições
    let mut filtered_chunks: Vec<String> = Vec::new();
    let mut filtered_positions: Vec<(usize, usize)> = Vec::new();
    let mut current_pos: usize = 0;

    for chunk in raw_chunks {
        let trimmed = chunk.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Encontrar posição do chunk no texto original (a partir da posição atual)
        if let Some(start_offset) = text[current_pos..].find(trimmed) {
            let start_pos = current_pos + start_offset;
            let end_pos = start_pos + trimmed.len();

            // Só incluir chunks que atendem ao tamanho mínimo
            if trimmed.len() >= options.min_chunk_length {
                filtered_chunks.push(trimmed.to_string());
                filtered_positions.push((start_pos, end_pos));
            }

            // Avançar posição para próxima busca
            current_pos = end_pos;
        }
    }

    ChunkResult {
        chunks: filtered_chunks,
        positions: filtered_positions,
    }
}

/// Split interno baseado no tipo de chunking
fn split_by_type(text: &str, chunk_type: &ChunkType) -> Vec<String> {
    match chunk_type {
        ChunkType::Newline => {
            text.split('\n')
                .map(|s| s.to_string())
                .collect()
        }

        ChunkType::Punctuation => {
            // Split por pontuação comum (preservando o delimitador)
            // Suporta: . ! ? 。 ！ ？
            // Rust regex não suporta look-behind, então usamos uma abordagem manual
            split_by_punctuation(text)
        }

        ChunkType::Characters(size) => {
            let size = *size;
            if size == 0 {
                return vec![text.to_string()];
            }

            let mut chunks = Vec::new();
            let chars: Vec<char> = text.chars().collect();

            for chunk_chars in chars.chunks(size) {
                chunks.push(chunk_chars.iter().collect());
            }

            chunks
        }

        ChunkType::Regex(pattern) => {
            match Regex::new(pattern) {
                Ok(re) => {
                    re.split(text)
                        .map(|s| s.to_string())
                        .collect()
                }
                Err(e) => {
                    log::warn!("Regex inválido '{}': {}", pattern, e);
                    vec![text.to_string()]
                }
            }
        }
    }
}

/// Split por pontuação (. ! ? 。 ！ ？) preservando o delimitador
fn split_by_punctuation(text: &str) -> Vec<String> {
    let punctuation = ['.', '!', '?', '。', '！', '？'];
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    for ch in text.chars() {
        current_chunk.push(ch);

        // Se encontramos pontuação, finalizar o chunk
        if punctuation.contains(&ch) {
            chunks.push(current_chunk.clone());
            current_chunk.clear();
        }
    }

    // Adicionar o que sobrou
    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    chunks
}

/// Encontra a posição de um chunk no texto, com busca a partir de offset
pub fn find_chunk_position(text: &str, chunk: &str, from: usize) -> Option<(usize, usize)> {
    text[from..].find(chunk).map(|offset| {
        let start = from + offset;
        let end = start + chunk.len();
        (start, end)
    })
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TESTES
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_by_newline() {
        let text = "Primeira linha com conteúdo suficiente para passar o filtro mínimo de oitenta caracteres.\nSegunda linha também com conteúdo suficiente para passar o filtro mínimo estabelecido.\nTerceira linha.";
        let result = chunk_text(text, &ChunkOptions::newline());

        // Apenas as duas primeiras linhas passam o filtro de 80 chars
        assert_eq!(result.chunks.len(), 2);
        assert!(result.chunks[0].contains("Primeira"));
        assert!(result.chunks[1].contains("Segunda"));
    }

    #[test]
    fn test_chunk_by_newline_with_small_min() {
        let text = "Linha 1\nLinha 2\nLinha 3";
        let result = chunk_text(text, &ChunkOptions::newline().with_min_length(5));

        assert_eq!(result.chunks.len(), 3);
    }

    #[test]
    fn test_chunk_by_punctuation() {
        let text = "Esta é a primeira sentença com conteúdo suficiente para validação. Esta é a segunda sentença também com conteúdo suficiente! E a terceira sentença? Sim, também passa.";
        let result = chunk_text(text, &ChunkOptions::punctuation().with_min_length(20));

        assert!(result.chunks.len() >= 2);
    }

    #[test]
    fn test_chunk_by_characters() {
        let text = "ABCDEFGHIJ1234567890ABCDEFGHIJ1234567890ABCDEFGHIJ"; // 50 chars
        let result = chunk_text(text, &ChunkOptions::characters(20).with_min_length(10));

        // 50 / 20 = 2 chunks completos + 1 de 10 chars
        assert!(result.chunks.len() >= 2);
    }

    #[test]
    fn test_chunk_by_regex() {
        let text = "Item1|||Item2|||Item3|||Item4 com conteúdo extra para passar|||Item5 também maior";
        let result = chunk_text(text, &ChunkOptions::regex(r"\|\|\|").with_min_length(10));

        assert!(result.chunks.len() >= 2);
    }

    #[test]
    fn test_chunk_positions_accuracy() {
        let text = "AAAAAAAAAA\nBBBBBBBBBB\nCCCCCCCCCC";
        let result = chunk_text(text, &ChunkOptions::newline().with_min_length(5));

        assert_eq!(result.chunks.len(), 3);

        // Verificar posições
        assert_eq!(result.positions[0], (0, 10));   // AAAAAAAAAA
        assert_eq!(result.positions[1], (11, 21));  // BBBBBBBBBB
        assert_eq!(result.positions[2], (22, 32));  // CCCCCCCCCC

        // Verificar que extrair do texto original bate
        for (chunk, (start, end)) in result.iter() {
            assert_eq!(chunk, &text[*start..*end]);
        }
    }

    #[test]
    fn test_empty_text() {
        let result = chunk_text("", &ChunkOptions::default());
        assert!(result.is_empty());
    }

    #[test]
    fn test_min_length_filter() {
        let text = "Curto\nEste texto é longo o suficiente para passar no filtro de oitenta caracteres mínimos\nOutro curto";
        let result = chunk_text(text, &ChunkOptions::newline());

        // Apenas o chunk longo passa
        assert_eq!(result.chunks.len(), 1);
        assert!(result.chunks[0].contains("longo o suficiente"));
    }

    #[test]
    fn test_chinese_punctuation() {
        let text = "这是第一句话，包含足够的内容来通过最小长度过滤器。这是第二句话！也有足够的内容？是的，当然有。";
        let result = chunk_text(text, &ChunkOptions::punctuation().with_min_length(10));

        // Deve separar por pontuação chinesa
        assert!(result.chunks.len() >= 1);
    }

    #[test]
    fn test_find_chunk_position() {
        let text = "Hello World Hello Universe";

        // Primeira ocorrência
        let pos1 = find_chunk_position(text, "Hello", 0);
        assert_eq!(pos1, Some((0, 5)));

        // Segunda ocorrência (buscando a partir de 6)
        let pos2 = find_chunk_position(text, "Hello", 6);
        assert_eq!(pos2, Some((12, 17)));
    }

    #[test]
    fn test_invalid_regex_fallback() {
        let text = "Some text here";
        // Regex inválido - deve retornar o texto original como único chunk
        let result = chunk_text(text, &ChunkOptions::regex("[invalid").with_min_length(5));

        assert_eq!(result.chunks.len(), 1);
        assert_eq!(result.chunks[0], text);
    }
}
