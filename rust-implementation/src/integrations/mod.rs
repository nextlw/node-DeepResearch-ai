//! # Integrações Externas
//!
//! Este módulo contém integrações com serviços externos que podem ser
//! usados pelo agente de pesquisa para executar ações específicas.
//!
//! ## Componentes
//!
//! - [`PaytourTools`]: Integração com API Paytour para passeios turísticos
//! - [`DigisacTools`]: Integração com API Digisac para mensagens WhatsApp

pub mod paytour_tools;
pub mod digisac_tools;

pub use paytour_tools::PaytourTools;
pub use digisac_tools::DigisacTools;
