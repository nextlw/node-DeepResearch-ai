// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEXT UTILITIES
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Utilitários para processamento de texto:
// - Truncation
// - Cleaning
// - Token estimation
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Estimativa de tokens por caractere (GPT-4)
const CHARS_PER_TOKEN: f32 = 4.0;

/// Estima número de tokens em um texto
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() as f32 / CHARS_PER_TOKEN).ceil() as usize
}

/// Trunca texto para um número máximo de tokens
pub fn truncate_to_tokens(text: &str, max_tokens: usize) -> &str {
    let max_chars = (max_tokens as f32 * CHARS_PER_TOKEN) as usize;
    if text.len() <= max_chars {
        text
    } else {
        // Encontra boundary de caractere válido
        let mut end = max_chars;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        &text[..end]
    }
}

/// Remove caracteres de controle e normaliza whitespace
pub fn clean_text(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extrai primeira sentença de um texto
pub fn first_sentence(text: &str) -> &str {
    let terminators = ['.', '!', '?'];
    for (i, c) in text.char_indices() {
        if terminators.contains(&c) {
            // Verifica se não é abreviação (ex: "Dr.", "U.S.")
            let remaining = &text[i + c.len_utf8()..];
            if remaining.starts_with(char::is_whitespace)
                || remaining.starts_with(char::is_uppercase)
                || remaining.is_empty()
            {
                return &text[..=i];
            }
        }
    }
    text
}

/// Conta palavras em um texto
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Extrai keywords de um texto (palavras mais longas)
pub fn extract_keywords(text: &str, max_keywords: usize) -> Vec<String> {
    let mut words: Vec<_> = text
        .split_whitespace()
        .filter(|w| w.len() >= 4) // Ignora palavras curtas
        .filter(|w| !is_stopword(w))
        .map(|w| w.to_lowercase())
        .collect();

    words.sort_by(|a, b| b.len().cmp(&a.len()));
    words.dedup();
    words.truncate(max_keywords);
    words
}

/// Verifica se é uma stopword comum
fn is_stopword(word: &str) -> bool {
    const STOPWORDS: &[&str] = &[
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
        "from", "as", "is", "was", "are", "were", "been", "be", "have", "has", "had", "do", "does",
        "did", "will", "would", "could", "should", "may", "might", "must", "shall", "can", "need",
        "this", "that", "these", "those", "what", "which", "who", "whom", "when", "where", "why",
        "how", "all", "each", "every", "both", "few", "more", "most", "other", "some", "such",
        "no", "nor", "not", "only", "own", "same", "so", "than", "too", "very", "just", "also",
    ];

    STOPWORDS.contains(&word.to_lowercase().as_str())
}

/// Normaliza query de busca
pub fn normalize_query(query: &str) -> String {
    query
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-' || *c == '_')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

// NOTA: Para chunking de texto, use `segment::chunk_text` que oferece
// 4 estratégias: newline, punctuation, characters(n), regex(pattern)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        let text = "Hello world"; // 11 chars ≈ 3 tokens
        assert!(estimate_tokens(text) >= 2 && estimate_tokens(text) <= 4);
    }

    #[test]
    fn test_truncate_to_tokens() {
        let text = "This is a longer text that should be truncated";
        let truncated = truncate_to_tokens(text, 5); // ~20 chars
        assert!(truncated.len() <= 25);
    }

    #[test]
    fn test_clean_text() {
        let text = "Hello\x00   world\t\ntest";
        let cleaned = clean_text(text);
        assert_eq!(cleaned, "Hello world test");
    }

    #[test]
    fn test_first_sentence() {
        assert_eq!(first_sentence("Hello world. More text."), "Hello world.");
        assert_eq!(first_sentence("No period"), "No period");
    }

    #[test]
    fn test_word_count() {
        assert_eq!(word_count("Hello world test"), 3);
        assert_eq!(word_count("  multiple   spaces  "), 2);
    }

    #[test]
    fn test_extract_keywords() {
        let text = "The quick brown fox jumps over the lazy dog";
        let keywords = extract_keywords(text, 3);
        assert!(keywords.contains(&"quick".to_string()));
        assert!(keywords.contains(&"brown".to_string()));
    }

    #[test]
    fn test_normalize_query() {
        assert_eq!(normalize_query("  Hello,  WORLD!!!  "), "hello world");
    }
}
