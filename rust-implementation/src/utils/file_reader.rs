//! # File Reader Utilities
//!
//! Este módulo fornece utilitários para download e leitura de arquivos de múltiplos
//! formatos, incluindo PDFs, documentos de texto, HTML, JSON, XML e Markdown.
//!
//! ## Funcionalidades Principais
//!
//! - **Download de arquivos**: Baixa arquivos de URLs com verificação de tamanho
//! - **Detecção de tipo**: Detecta automaticamente o tipo de arquivo pela extensão ou content-type
//! - **Extração de texto**: Extrai conteúdo textual de diferentes formatos (PDF, HTML, etc.)
//! - **Leitura local**: Suporte para leitura de arquivos do sistema de arquivos local
//!
//! ## Exemplo de Uso
//!
//! ```rust,no_run
//! use crate::utils::file_reader::{FileReader, FileType};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let reader = FileReader::new();
//!
//!     // Baixar e processar um PDF
//!     let content = reader.read_url("https://example.com/document.pdf").await?;
//!     println!("Palavras extraídas: {}", content.word_count);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Limites
//!
//! - Tamanho máximo de arquivo: 100MB (configurável via [`FileReader::with_max_size`])
//! - Timeout de download: 60 segundos
//!
//! ## Tipos de Arquivo Suportados
//!
//! | Tipo | Extensões | Extração de Texto |
//! |------|-----------|-------------------|
//! | PDF | `.pdf` | ✅ Sim |
//! | HTML | `.html`, `.htm` | ✅ Sim |
//! | Texto | `.txt` | ✅ Sim |
//! | Markdown | `.md`, `.markdown` | ✅ Sim |
//! | JSON | `.json` | ✅ Sim |
//! | XML | `.xml` | ✅ Sim |
//! | Imagem | `.png`, `.jpg`, `.gif`, `.webp` | ❌ Não |

use thiserror::Error;

/// Limite máximo de tamanho de arquivo padrão (100MB).
///
/// Este valor pode ser sobrescrito usando [`FileReader::with_max_size`].
const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

/// Erros que podem ocorrer durante operações de leitura de arquivos.
///
/// Este enum representa todos os possíveis erros que podem ocorrer ao baixar,
/// ler ou processar arquivos usando o [`FileReader`].
///
/// # Variantes
///
/// - [`DownloadError`](FileReaderError::DownloadError) - Falha no download HTTP
/// - [`FileTooLarge`](FileReaderError::FileTooLarge) - Arquivo excede o limite de tamanho
/// - [`UnsupportedType`](FileReaderError::UnsupportedType) - Tipo de arquivo não suportado
/// - [`PdfExtractionError`](FileReaderError::PdfExtractionError) - Erro ao extrair texto de PDF
/// - [`IoError`](FileReaderError::IoError) - Erro de I/O do sistema de arquivos
/// - [`NetworkError`](FileReaderError::NetworkError) - Erro de rede/conexão
///
/// # Exemplo
///
/// ```rust
/// use crate::utils::file_reader::FileReaderError;
///
/// fn handle_error(err: FileReaderError) {
///     match err {
///         FileReaderError::FileTooLarge { size, max } => {
///             eprintln!("Arquivo muito grande: {} bytes (máximo: {})", size, max);
///         }
///         FileReaderError::NetworkError(msg) => {
///             eprintln!("Erro de rede: {}", msg);
///         }
///         _ => eprintln!("Erro: {}", err),
///     }
/// }
/// ```
#[derive(Debug, Error)]
pub enum FileReaderError {
    /// Falha durante o download do arquivo.
    ///
    /// Ocorre quando o servidor retorna um código de status HTTP de erro
    /// ou quando há problemas ao receber os bytes do arquivo.
    #[error("Download failed: {0}")]
    DownloadError(String),

    /// Arquivo excede o limite máximo de tamanho permitido.
    ///
    /// O campo `size` contém o tamanho do arquivo em bytes e `max` contém
    /// o limite configurado.
    #[error("File too large: {size} bytes (max: {max})")]
    FileTooLarge {
        /// Tamanho do arquivo em bytes
        size: u64,
        /// Limite máximo permitido em bytes
        max: u64,
    },

    /// Tipo de arquivo não suportado para extração de texto.
    ///
    /// Ocorre ao tentar extrair texto de formatos que não suportam
    /// conversão para texto, como imagens.
    #[error("Unsupported file type: {0}")]
    UnsupportedType(String),

    /// Falha ao extrair texto de um arquivo PDF.
    ///
    /// Pode ocorrer quando o PDF está corrompido, protegido por senha,
    /// ou contém apenas imagens escaneadas sem OCR.
    #[error("PDF extraction failed: {0}")]
    PdfExtractionError(String),

    /// Erro de entrada/saída do sistema de arquivos.
    ///
    /// Ocorre durante operações de leitura de arquivos locais.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Erro de rede ou conexão HTTP.
    ///
    /// Ocorre quando há problemas de conectividade, timeout,
    /// ou erros de resolução DNS.
    #[error("Network error: {0}")]
    NetworkError(String),
}

/// Representa os tipos de arquivo suportados pelo leitor.
///
/// Este enum é usado para identificar o formato de um arquivo, seja através
/// da extensão da URL ou do header Content-Type HTTP. A detecção do tipo
/// determina como o conteúdo será processado e extraído.
///
/// # Variantes
///
/// | Variante | Descrição | Extração |
/// |----------|-----------|----------|
/// | `Pdf` | Documento PDF | Usa `pdf_extract` |
/// | `Html` | Página HTML | UTF-8 direto |
/// | `Text` | Texto puro | UTF-8 direto |
/// | `Markdown` | Documento Markdown | UTF-8 direto |
/// | `Json` | Dados JSON | UTF-8 direto |
/// | `Xml` | Documento XML | UTF-8 direto |
/// | `Image` | Arquivo de imagem | Não suportado |
/// | `Unknown` | Tipo desconhecido | Tenta como texto |
///
/// # Exemplo
///
/// ```rust
/// use crate::utils::file_reader::FileType;
///
/// let file_type = FileType::from_url("https://example.com/doc.pdf");
/// assert_eq!(file_type, FileType::Pdf);
///
/// let content_type = FileType::from_content_type("application/json");
/// assert_eq!(content_type, FileType::Json);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    /// Documento PDF (Portable Document Format).
    ///
    /// Arquivos com extensão `.pdf` ou content-type `application/pdf`.
    Pdf,

    /// Documento HTML (HyperText Markup Language).
    ///
    /// Arquivos com extensão `.html` ou `.htm`, ou content-type `text/html`.
    Html,

    /// Arquivo de texto puro.
    ///
    /// Arquivos com extensão `.txt` ou content-type `text/plain`.
    Text,

    /// Documento Markdown.
    ///
    /// Arquivos com extensão `.md` ou `.markdown`, ou content-type `text/markdown`.
    Markdown,

    /// Dados no formato JSON (JavaScript Object Notation).
    ///
    /// Arquivos com extensão `.json` ou content-type `application/json`.
    Json,

    /// Documento XML (eXtensible Markup Language).
    ///
    /// Arquivos com extensão `.xml` ou content-type `application/xml` ou `text/xml`.
    Xml,

    /// Arquivo de imagem (PNG, JPEG, GIF, WebP).
    ///
    /// **Nota**: Extração de texto não é suportada para imagens.
    Image,

    /// Tipo de arquivo desconhecido ou não mapeado.
    ///
    /// O [`String`] interno contém o content-type original (se disponível).
    /// A extração tenta processar como texto UTF-8.
    Unknown(String),
}

impl FileType {
    /// Detecta o tipo de arquivo pela extensão presente na URL.
    ///
    /// Analisa a URL fornecida e identifica o tipo de arquivo baseado
    /// na extensão do arquivo (case-insensitive).
    ///
    /// # Argumentos
    ///
    /// * `url` - URL completa ou caminho do arquivo a ser analisado
    ///
    /// # Retorno
    ///
    /// Retorna a variante [`FileType`] correspondente à extensão detectada,
    /// ou [`FileType::Unknown`] se a extensão não for reconhecida.
    ///
    /// # Extensões Suportadas
    ///
    /// - `.pdf` → [`FileType::Pdf`]
    /// - `.html`, `.htm` → [`FileType::Html`]
    /// - `.txt` → [`FileType::Text`]
    /// - `.md`, `.markdown` → [`FileType::Markdown`]
    /// - `.json` → [`FileType::Json`]
    /// - `.xml` → [`FileType::Xml`]
    /// - `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp` → [`FileType::Image`]
    ///
    /// # Exemplo
    ///
    /// ```rust
    /// use crate::utils::file_reader::FileType;
    ///
    /// assert_eq!(FileType::from_url("https://example.com/doc.PDF"), FileType::Pdf);
    /// assert_eq!(FileType::from_url("/local/path/readme.md"), FileType::Markdown);
    /// assert_eq!(FileType::from_url("https://api.example.com/data"), FileType::Unknown(String::new()));
    /// ```
    pub fn from_url(url: &str) -> Self {
        let url_lower = url.to_lowercase();

        if url_lower.ends_with(".pdf") {
            Self::Pdf
        } else if url_lower.ends_with(".html") || url_lower.ends_with(".htm") {
            Self::Html
        } else if url_lower.ends_with(".txt") {
            Self::Text
        } else if url_lower.ends_with(".md") || url_lower.ends_with(".markdown") {
            Self::Markdown
        } else if url_lower.ends_with(".json") {
            Self::Json
        } else if url_lower.ends_with(".xml") {
            Self::Xml
        } else if url_lower.ends_with(".png")
            || url_lower.ends_with(".jpg")
            || url_lower.ends_with(".jpeg")
            || url_lower.ends_with(".gif")
            || url_lower.ends_with(".webp")
        {
            Self::Image
        } else {
            Self::Unknown(String::new())
        }
    }

    /// Detecta o tipo de arquivo pelo header Content-Type HTTP.
    ///
    /// Analisa o valor do header Content-Type e identifica o tipo de arquivo
    /// correspondente (case-insensitive). Suporta content-types com parâmetros
    /// adicionais como charset.
    ///
    /// # Argumentos
    ///
    /// * `content_type` - Valor do header Content-Type (ex: "application/pdf", "text/html; charset=utf-8")
    ///
    /// # Retorno
    ///
    /// Retorna a variante [`FileType`] correspondente ao content-type,
    /// ou [`FileType::Unknown`] contendo o content-type original se não reconhecido.
    ///
    /// # Content-Types Suportados
    ///
    /// - `application/pdf` → [`FileType::Pdf`]
    /// - `text/html` → [`FileType::Html`]
    /// - `text/plain` → [`FileType::Text`]
    /// - `text/markdown` → [`FileType::Markdown`]
    /// - `application/json` → [`FileType::Json`]
    /// - `application/xml`, `text/xml` → [`FileType::Xml`]
    /// - `image/*` → [`FileType::Image`]
    ///
    /// # Exemplo
    ///
    /// ```rust
    /// use crate::utils::file_reader::FileType;
    ///
    /// assert_eq!(FileType::from_content_type("application/pdf"), FileType::Pdf);
    /// assert_eq!(FileType::from_content_type("text/html; charset=utf-8"), FileType::Html);
    /// assert_eq!(FileType::from_content_type("image/png"), FileType::Image);
    /// ```
    pub fn from_content_type(content_type: &str) -> Self {
        let ct_lower = content_type.to_lowercase();

        if ct_lower.contains("application/pdf") {
            Self::Pdf
        } else if ct_lower.contains("text/html") {
            Self::Html
        } else if ct_lower.contains("text/plain") {
            Self::Text
        } else if ct_lower.contains("text/markdown") {
            Self::Markdown
        } else if ct_lower.contains("application/json") {
            Self::Json
        } else if ct_lower.contains("application/xml") || ct_lower.contains("text/xml") {
            Self::Xml
        } else if ct_lower.starts_with("image/") {
            Self::Image
        } else {
            Self::Unknown(content_type.to_string())
        }
    }
}

/// Resultado de uma operação de leitura de arquivo.
///
/// Esta struct contém todo o conteúdo extraído de um arquivo, junto com
/// metadados úteis como tipo de arquivo, tamanho e contagem de palavras.
///
/// # Campos
///
/// | Campo | Descrição |
/// |-------|-----------|
/// | `source` | URL ou caminho original do arquivo |
/// | `file_type` | Tipo detectado do arquivo |
/// | `text` | Conteúdo textual extraído |
/// | `title` | Título do documento (quando disponível) |
/// | `size_bytes` | Tamanho original em bytes |
/// | `word_count` | Número de palavras no texto extraído |
/// | `metadata` | Metadados adicionais (ex: autor, data de criação) |
///
/// # Exemplo
///
/// ```rust,no_run
/// use crate::utils::file_reader::FileReader;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let reader = FileReader::new();
///     let content = reader.read_url("https://example.com/document.pdf").await?;
///
///     println!("Arquivo: {}", content.source);
///     println!("Tipo: {:?}", content.file_type);
///     println!("Tamanho: {} bytes", content.size_bytes);
///     println!("Palavras: {}", content.word_count);
///     println!("Conteúdo (primeiros 100 chars): {}", &content.text[..100.min(content.text.len())]);
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct FileContent {
    /// URL ou caminho original do arquivo de onde o conteúdo foi extraído.
    ///
    /// Pode ser uma URL HTTP/HTTPS ou um caminho do sistema de arquivos local.
    pub source: String,

    /// Tipo de arquivo detectado durante o processamento.
    ///
    /// Veja [`FileType`] para a lista completa de tipos suportados.
    pub file_type: FileType,

    /// Conteúdo textual extraído do arquivo.
    ///
    /// Para PDFs, contém o texto extraído. Para outros formatos de texto,
    /// contém o conteúdo bruto. Para tipos não suportados, estará vazio.
    pub text: String,

    /// Título do documento, quando disponível.
    ///
    /// Atualmente não preenchido automaticamente; reservado para
    /// extração futura de metadados de PDFs.
    pub title: Option<String>,

    /// Tamanho original do arquivo em bytes (antes da extração de texto).
    pub size_bytes: u64,

    /// Número de palavras no texto extraído.
    ///
    /// Calculado usando [`str::split_whitespace`], útil para estimativas
    /// de tempo de leitura ou limites de processamento.
    pub word_count: usize,

    /// Metadados adicionais extraídos do documento.
    ///
    /// Pode conter informações como autor, data de criação, versão do PDF, etc.
    /// Atualmente não preenchido automaticamente; reservado para uso futuro.
    pub metadata: std::collections::HashMap<String, String>,
}

/// Leitor de arquivos com suporte a múltiplos formatos.
///
/// O `FileReader` é a principal estrutura para download e processamento de arquivos.
/// Suporta leitura tanto de URLs remotas quanto de arquivos locais, com detecção
/// automática de tipo e extração de texto.
///
/// # Funcionalidades
///
/// - Download assíncrono de arquivos via HTTP/HTTPS
/// - Leitura síncrona de arquivos locais
/// - Verificação de tamanho máximo antes do download
/// - Extração de texto de PDFs usando `pdf_extract`
/// - Processamento de arquivos de texto (HTML, Markdown, JSON, XML)
///
/// # Configuração
///
/// O leitor pode ser configurado usando o padrão builder:
///
/// ```rust
/// use crate::utils::file_reader::FileReader;
///
/// let reader = FileReader::new()
///     .with_max_size(50 * 1024 * 1024); // 50MB
/// ```
///
/// # Exemplo de Uso Completo
///
/// ```rust,no_run
/// use crate::utils::file_reader::FileReader;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let reader = FileReader::new();
///
///     // Verificar se URL é baixável
///     if FileReader::is_downloadable_url("https://example.com/doc.pdf") {
///         let content = reader.read_url("https://example.com/doc.pdf").await?;
///         println!("Extraídas {} palavras do PDF", content.word_count);
///     }
///
///     // Ler arquivo local
///     let local = reader.read_file("/path/to/local/file.txt")?;
///     println!("Conteúdo: {}", local.text);
///
///     Ok(())
/// }
/// ```
///
/// # Erros
///
/// Todas as operações podem retornar [`FileReaderError`] em caso de falha.
/// Consulte a documentação do enum para detalhes sobre cada tipo de erro.
///
/// # Thread Safety
///
/// O `FileReader` é thread-safe e pode ser compartilhado entre threads
/// usando `Arc<FileReader>`.
pub struct FileReader {
    /// Cliente HTTP para downloads remotos
    client: reqwest::Client,
    /// Tamanho máximo de arquivo permitido em bytes
    max_size: u64,
}

impl FileReader {
    /// Cria uma nova instância do `FileReader` com configurações padrão.
    ///
    /// # Configurações Padrão
    ///
    /// - **Timeout**: 60 segundos para operações HTTP
    /// - **Tamanho máximo**: 100MB ([`MAX_FILE_SIZE`])
    ///
    /// # Exemplo
    ///
    /// ```rust
    /// use crate::utils::file_reader::FileReader;
    ///
    /// let reader = FileReader::new();
    /// ```
    ///
    /// # Panics
    ///
    /// Não causa panic. Se a construção do cliente HTTP falhar,
    /// um cliente padrão é usado como fallback.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
            max_size: MAX_FILE_SIZE,
        }
    }

    /// Define o tamanho máximo de arquivo permitido para download.
    ///
    /// Este método consome `self` e retorna uma nova instância configurada,
    /// permitindo encadeamento de métodos (builder pattern).
    ///
    /// # Argumentos
    ///
    /// * `max_size` - Tamanho máximo em bytes
    ///
    /// # Exemplo
    ///
    /// ```rust
    /// use crate::utils::file_reader::FileReader;
    ///
    /// // Permitir arquivos de até 50MB
    /// let reader = FileReader::new()
    ///     .with_max_size(50 * 1024 * 1024);
    ///
    /// // Permitir arquivos de até 1GB
    /// let large_reader = FileReader::new()
    ///     .with_max_size(1024 * 1024 * 1024);
    /// ```
    pub fn with_max_size(mut self, max_size: u64) -> Self {
        self.max_size = max_size;
        self
    }

    /// Baixa um arquivo de uma URL remota.
    ///
    /// Realiza uma requisição HTTP GET para a URL especificada e retorna
    /// os bytes do arquivo. Verifica o tamanho do arquivo antes de baixar
    /// (se o servidor fornecer Content-Length) para evitar downloads de
    /// arquivos muito grandes.
    ///
    /// # Argumentos
    ///
    /// * `url` - URL completa do arquivo a ser baixado
    ///
    /// # Retorno
    ///
    /// Retorna `Ok(Vec<u8>)` com os bytes do arquivo em caso de sucesso.
    ///
    /// # Erros
    ///
    /// - [`FileReaderError::NetworkError`] - Falha de conexão ou timeout
    /// - [`FileReaderError::FileTooLarge`] - Arquivo excede o limite configurado
    /// - [`FileReaderError::DownloadError`] - Servidor retornou erro HTTP
    ///
    /// # Exemplo
    ///
    /// ```rust,no_run
    /// use crate::utils::file_reader::FileReader;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let reader = FileReader::new();
    ///     let bytes = reader.download("https://example.com/file.pdf").await?;
    ///     println!("Baixados {} bytes", bytes.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn download(&self, url: &str) -> Result<Vec<u8>, FileReaderError> {
        log::info!("📥 Baixando arquivo: {}", url);

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| FileReaderError::NetworkError(e.to_string()))?;

        // Verificar tamanho antes de baixar
        if let Some(content_length) = response.content_length() {
            if content_length > self.max_size {
                return Err(FileReaderError::FileTooLarge {
                    size: content_length,
                    max: self.max_size,
                });
            }
        }

        if !response.status().is_success() {
            return Err(FileReaderError::DownloadError(format!(
                "HTTP {}: {}",
                response.status(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| FileReaderError::DownloadError(e.to_string()))?;

        log::info!("✅ Download concluído: {} bytes", bytes.len());
        Ok(bytes.to_vec())
    }

    /// Extrai texto de um arquivo PDF em memória.
    ///
    /// Utiliza a biblioteca `pdf_extract` para extrair o conteúdo textual
    /// de um documento PDF. Esta é uma função estática que pode ser chamada
    /// sem uma instância de `FileReader`.
    ///
    /// # Argumentos
    ///
    /// * `data` - Slice de bytes contendo o arquivo PDF completo
    ///
    /// # Retorno
    ///
    /// Retorna `Ok(String)` com o texto extraído do PDF.
    ///
    /// # Erros
    ///
    /// Retorna [`FileReaderError::PdfExtractionError`] se:
    /// - O PDF estiver corrompido ou mal-formado
    /// - O PDF estiver protegido por senha
    /// - O PDF contiver apenas imagens (sem texto pesquisável)
    /// - Ocorrer erro interno na biblioteca de extração
    ///
    /// # Exemplo
    ///
    /// ```rust,no_run
    /// use crate::utils::file_reader::FileReader;
    ///
    /// let pdf_bytes = std::fs::read("document.pdf")?;
    /// let text = FileReader::extract_pdf_text(&pdf_bytes)?;
    /// println!("Texto extraído: {}", text);
    /// ```
    ///
    /// # Limitações
    ///
    /// - PDFs escaneados (imagens) não terão texto extraído sem OCR
    /// - A formatação original (tabelas, colunas) pode ser perdida
    /// - Alguns caracteres especiais podem não ser extraídos corretamente
    pub fn extract_pdf_text(data: &[u8]) -> Result<String, FileReaderError> {
        log::info!("📄 Extraindo texto de PDF ({} bytes)", data.len());

        pdf_extract::extract_text_from_mem(data)
            .map_err(|e| FileReaderError::PdfExtractionError(e.to_string()))
    }

    /// Lê e processa um arquivo de uma URL remota.
    ///
    /// Este método combina download e processamento em uma única operação.
    /// O tipo de arquivo é detectado automaticamente pela extensão da URL.
    ///
    /// # Argumentos
    ///
    /// * `url` - URL completa do arquivo a ser lido
    ///
    /// # Retorno
    ///
    /// Retorna [`FileContent`] contendo o texto extraído e metadados.
    ///
    /// # Erros
    ///
    /// - Todos os erros de [`download`](FileReader::download)
    /// - [`FileReaderError::PdfExtractionError`] - Falha ao extrair texto de PDF
    /// - [`FileReaderError::UnsupportedType`] - Tipo de arquivo não suporta extração
    ///
    /// # Exemplo
    ///
    /// ```rust,no_run
    /// use crate::utils::file_reader::FileReader;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let reader = FileReader::new();
    ///     let content = reader.read_url("https://example.com/document.pdf").await?;
    ///
    ///     println!("Tipo: {:?}", content.file_type);
    ///     println!("Palavras: {}", content.word_count);
    ///     println!("Preview: {}...", &content.text[..200.min(content.text.len())]);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn read_url(&self, url: &str) -> Result<FileContent, FileReaderError> {
        let file_type = FileType::from_url(url);
        let data = self.download(url).await?;

        self.process_content(url, &data, file_type)
    }

    /// Lê e processa um arquivo do sistema de arquivos local.
    ///
    /// Este método realiza leitura síncrona do arquivo e processa seu conteúdo.
    /// O tipo de arquivo é detectado automaticamente pela extensão do caminho.
    ///
    /// # Argumentos
    ///
    /// * `path` - Caminho absoluto ou relativo do arquivo a ser lido
    ///
    /// # Retorno
    ///
    /// Retorna [`FileContent`] contendo o texto extraído e metadados.
    ///
    /// # Erros
    ///
    /// - [`FileReaderError::IoError`] - Arquivo não encontrado ou sem permissão
    /// - [`FileReaderError::PdfExtractionError`] - Falha ao extrair texto de PDF
    /// - [`FileReaderError::UnsupportedType`] - Tipo de arquivo não suporta extração
    ///
    /// # Exemplo
    ///
    /// ```rust,no_run
    /// use crate::utils::file_reader::FileReader;
    ///
    /// let reader = FileReader::new();
    /// let content = reader.read_file("/path/to/document.txt")?;
    ///
    /// println!("Tamanho: {} bytes", content.size_bytes);
    /// println!("Conteúdo: {}", content.text);
    /// ```
    pub fn read_file(&self, path: &str) -> Result<FileContent, FileReaderError> {
        log::info!("📂 Lendo arquivo local: {}", path);

        let data = std::fs::read(path)?;
        let file_type = FileType::from_url(path);

        self.process_content(path, &data, file_type)
    }

    /// Processa o conteúdo de um arquivo e extrai texto.
    ///
    /// Este é um método interno que realiza a extração de texto baseado
    /// no tipo de arquivo detectado.
    ///
    /// # Argumentos
    ///
    /// * `source` - URL ou caminho original do arquivo (para referência)
    /// * `data` - Bytes brutos do arquivo
    /// * `file_type` - Tipo de arquivo detectado
    ///
    /// # Retorno
    ///
    /// Retorna [`FileContent`] com o texto extraído e metadados calculados.
    ///
    /// # Estratégia de Processamento
    ///
    /// - **PDF**: Usa [`extract_pdf_text`](FileReader::extract_pdf_text)
    /// - **Text/Markdown/HTML/JSON/XML**: Converte bytes para UTF-8
    /// - **Image**: Retorna erro (não suportado)
    /// - **Unknown**: Tenta converter para UTF-8 como fallback
    fn process_content(
        &self,
        source: &str,
        data: &[u8],
        file_type: FileType,
    ) -> Result<FileContent, FileReaderError> {
        let size_bytes = data.len() as u64;

        let (text, title) = match &file_type {
            FileType::Pdf => (Self::extract_pdf_text(data)?, None),
            FileType::Html => {
                // Extrair texto limpo do HTML
                let (extracted_text, extracted_title) = Self::extract_html_text(data);
                (extracted_text, extracted_title)
            }
            FileType::Text | FileType::Markdown => {
                (String::from_utf8_lossy(data).to_string(), None)
            }
            FileType::Json | FileType::Xml => (String::from_utf8_lossy(data).to_string(), None),
            FileType::Image => {
                return Err(FileReaderError::UnsupportedType(
                    "Images cannot be converted to text".into(),
                ));
            }
            FileType::Unknown(_ct) => {
                // Tentar como HTML se parecer com HTML, senão como texto
                let raw = String::from_utf8_lossy(data).to_string();
                if raw.contains("<html") || raw.contains("<body") || raw.contains("<!DOCTYPE") {
                    let (extracted_text, extracted_title) = Self::extract_html_text(data);
                    (extracted_text, extracted_title)
                } else {
                    (raw, None)
                }
            }
        };

        let word_count = text.split_whitespace().count();

        log::info!(
            "✅ Arquivo processado: {} | tipo={:?} | {} bytes | {} palavras{}",
            source,
            file_type,
            size_bytes,
            word_count,
            title.as_ref().map(|t| format!(" | título: {}", t)).unwrap_or_default()
        );

        Ok(FileContent {
            source: source.to_string(),
            file_type,
            text,
            title,
            size_bytes,
            word_count,
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Extrai texto limpo de HTML usando Mozilla Readability algorithm.
    ///
    /// Este é o mesmo algoritmo usado pelo Firefox Reader Mode e pelo Jina Reader.
    /// Automaticamente identifica e extrai o conteúdo principal, removendo:
    /// - Navegação, headers, footers
    /// - Anúncios e sidebars
    /// - Scripts e estilos
    /// - Elementos não relacionados ao conteúdo
    ///
    /// # Retorna
    /// (texto_extraído, título_opcional)
    fn extract_html_text(data: &[u8]) -> (String, Option<String>) {
        use readability::extractor;

        let html_str = String::from_utf8_lossy(data).to_string();

        // Tentar usar Mozilla Readability primeiro
        match extractor::extract(&mut html_str.as_bytes(), &url::Url::parse("https://example.com").unwrap()) {
            Ok(product) => {
                let title = if product.title.is_empty() {
                    None
                } else {
                    Some(product.title)
                };

                // O readability retorna HTML limpo, precisamos converter para texto
                let clean_text = Self::html_to_plain_text(&product.content);

                log::debug!(
                    "📖 Readability extraiu: {} chars | título: {:?}",
                    clean_text.len(),
                    title
                );

                (clean_text, title)
            }
            Err(e) => {
                log::warn!("⚠️ Readability falhou: {}, usando fallback html2text", e);
                // Fallback para html2text
                Self::extract_html_text_fallback(&html_str)
            }
        }
    }

    /// Converte HTML limpo para texto puro usando html2text com fallback para strip_html_tags
    fn html_to_plain_text(html: &str) -> String {
        // Tentar html2text primeiro
        let text = html2text::from_read(html.as_bytes(), 120);
        let cleaned = Self::clean_extracted_text(&text);

        // Se html2text retornar muito pouco, usar strip_html_tags como fallback
        if cleaned.len() < 50 {
            log::debug!(
                "html2text retornou pouco texto ({} chars), usando strip_html_tags",
                cleaned.len()
            );
            let stripped = Self::strip_html_tags(html);
            if stripped.len() > cleaned.len() {
                return stripped;
            }
        }

        cleaned
    }

    /// Fallback: extrai texto de HTML quando Readability falha
    fn extract_html_text_fallback(html: &str) -> (String, Option<String>) {
        // Tentar html2text primeiro
        let text = html2text::from_read(html.as_bytes(), 120);
        let cleaned = Self::clean_extracted_text(&text);

        // Se retornou muito pouco, tentar strip_html_tags
        let final_text = if cleaned.len() < 50 {
            let stripped = Self::strip_html_tags(html);
            if stripped.len() > cleaned.len() {
                stripped
            } else {
                cleaned
            }
        } else {
            cleaned
        };

        // Tentar extrair título manualmente
        let title = Self::extract_title_from_html(html);

        (final_text, title)
    }

    /// Remove tags HTML de forma básica (último fallback)
    ///
    /// Usa uma máquina de estados para:
    /// - Remover todas as tags HTML
    /// - Ignorar conteúdo de <script> e <style>
    /// - Preservar apenas texto visível
    fn strip_html_tags(html: &str) -> String {
        let mut result = String::with_capacity(html.len() / 2);
        let mut in_tag = false;
        let mut in_script = false;
        let mut in_style = false;
        let mut tag_buffer = String::new();

        for c in html.chars() {
            match c {
                '<' => {
                    in_tag = true;
                    tag_buffer.clear();
                }
                '>' => {
                    in_tag = false;
                    let tag_lower = tag_buffer.to_lowercase();

                    // Detectar início/fim de script e style
                    if tag_lower.starts_with("script") {
                        in_script = true;
                    } else if tag_lower.starts_with("/script") {
                        in_script = false;
                    } else if tag_lower.starts_with("style") {
                        in_style = true;
                    } else if tag_lower.starts_with("/style") {
                        in_style = false;
                    }
                    // Adicionar espaço após certas tags de bloco
                    else if tag_lower.starts_with("br")
                        || tag_lower.starts_with("p")
                        || tag_lower.starts_with("/p")
                        || tag_lower.starts_with("div")
                        || tag_lower.starts_with("/div")
                        || tag_lower.starts_with("li")
                        || tag_lower.starts_with("h")
                    {
                        result.push(' ');
                    }

                    tag_buffer.clear();
                }
                _ if in_tag => {
                    tag_buffer.push(c);
                }
                _ if !in_script && !in_style => {
                    // Converter entidades HTML comuns
                    result.push(c);
                }
                _ => {}
            }
        }

        // Decodificar entidades HTML básicas
        let decoded = result
            .replace("&nbsp;", " ")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'");

        Self::clean_extracted_text(&decoded)
    }

    /// Extrai título de HTML manualmente
    fn extract_title_from_html(html: &str) -> Option<String> {
        // Buscar <title>...</title>
        let lower = html.to_lowercase();
        if let Some(start) = lower.find("<title>") {
            if let Some(end) = lower[start..].find("</title>") {
                let title_start = start + 7; // len("<title>")
                let title_end = start + end;
                if title_end > title_start && title_end <= html.len() {
                    let title = html[title_start..title_end].trim().to_string();
                    if !title.is_empty() {
                        return Some(title);
                    }
                }
            }
        }
        None
    }

    /// Limpa texto extraído removendo espaços extras e linhas vazias
    fn clean_extracted_text(text: &str) -> String {
        let lines: Vec<&str> = text
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && l.len() > 1) // Ignorar linhas muito curtas
            .collect();

        // Remover linhas duplicadas consecutivas
        let mut result: Vec<&str> = Vec::new();
        for line in lines {
            if result.last() != Some(&line) {
                result.push(line);
            }
        }

        // Juntar com quebras de linha e limpar múltiplos espaços
        result
            .join("\n")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Verifica se uma URL aponta para um arquivo que pode ser baixado e processado.
    ///
    /// Esta função utilitária permite verificar rapidamente se uma URL
    /// corresponde a um tipo de arquivo suportado para extração de texto,
    /// antes de realizar o download.
    ///
    /// # Argumentos
    ///
    /// * `url` - URL a ser verificada
    ///
    /// # Retorno
    ///
    /// Retorna `true` se o arquivo pode ser baixado e processado:
    /// - PDF (`.pdf`)
    /// - Texto (`.txt`)
    /// - Markdown (`.md`, `.markdown`)
    /// - JSON (`.json`)
    /// - XML (`.xml`)
    ///
    /// Retorna `false` para:
    /// - HTML (páginas web devem usar scraping)
    /// - Imagens
    /// - Tipos desconhecidos
    ///
    /// # Exemplo
    ///
    /// ```rust
    /// use crate::utils::file_reader::FileReader;
    ///
    /// assert!(FileReader::is_downloadable_url("https://example.com/doc.pdf"));
    /// assert!(FileReader::is_downloadable_url("https://api.example.com/data.json"));
    /// assert!(!FileReader::is_downloadable_url("https://example.com/image.png"));
    /// assert!(!FileReader::is_downloadable_url("https://example.com/page.html"));
    /// ```
    ///
    /// # Uso Típico
    ///
    /// ```rust,no_run
    /// use crate::utils::file_reader::FileReader;
    ///
    /// fn process_link(url: &str) {
    ///     if FileReader::is_downloadable_url(url) {
    ///         // Usar FileReader para baixar e extrair texto
    ///     } else {
    ///         // Usar web scraper ou outro método
    ///     }
    /// }
    /// ```
    pub fn is_downloadable_url(url: &str) -> bool {
        let file_type = FileType::from_url(url);
        matches!(
            file_type,
            FileType::Pdf | FileType::Text | FileType::Markdown | FileType::Json | FileType::Xml
        )
    }
}

impl Default for FileReader {
    /// Cria uma instância padrão do `FileReader`.
    ///
    /// Equivalente a chamar [`FileReader::new()`].
    ///
    /// # Exemplo
    ///
    /// ```rust
    /// use crate::utils::file_reader::FileReader;
    ///
    /// let reader = FileReader::default();
    /// // Equivalente a:
    /// let reader2 = FileReader::new();
    /// ```
    fn default() -> Self {
        Self::new()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TESTES UNITÁRIOS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    /// Testa a detecção de tipo de arquivo por extensão da URL.
    ///
    /// Verifica se [`FileType::from_url`] detecta corretamente os tipos
    /// de arquivo baseado nas extensões mais comuns.
    #[test]
    fn test_file_type_detection() {
        assert_eq!(
            FileType::from_url("https://example.com/doc.pdf"),
            FileType::Pdf
        );
        assert_eq!(
            FileType::from_url("https://example.com/page.html"),
            FileType::Html
        );
        assert_eq!(
            FileType::from_url("https://example.com/readme.md"),
            FileType::Markdown
        );
        assert_eq!(
            FileType::from_url("https://example.com/data.json"),
            FileType::Json
        );
    }

    /// Testa a detecção de tipo de arquivo pelo header Content-Type.
    ///
    /// Verifica se [`FileType::from_content_type`] detecta corretamente
    /// os tipos de arquivo, incluindo content-types com parâmetros (charset).
    #[test]
    fn test_content_type_detection() {
        assert_eq!(
            FileType::from_content_type("application/pdf"),
            FileType::Pdf
        );
        assert_eq!(
            FileType::from_content_type("text/html; charset=utf-8"),
            FileType::Html
        );
        assert_eq!(FileType::from_content_type("text/plain"), FileType::Text);
    }

    /// Testa a verificação de URLs baixáveis.
    ///
    /// Verifica se [`FileReader::is_downloadable_url`] retorna `true` para
    /// tipos suportados e `false` para tipos não suportados (imagens).
    #[test]
    fn test_is_downloadable() {
        assert!(FileReader::is_downloadable_url(
            "https://example.com/doc.pdf"
        ));
        assert!(FileReader::is_downloadable_url(
            "https://example.com/data.json"
        ));
        assert!(!FileReader::is_downloadable_url(
            "https://example.com/image.png"
        ));
    }
}
