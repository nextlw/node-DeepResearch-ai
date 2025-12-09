/**
 * 🏎️ Benchmark TypeScript - DeepResearch AI
 * 
 * Mede operações reais para comparar com Rust
 */

// Cosine similarity - igual ao src/tools/cosine.ts
function cosineSimilarity(vecA: number[], vecB: number[]): number {
  let dotProduct = 0;
  let magnitudeA = 0;
  let magnitudeB = 0;

  for (let i = 0; i < vecA.length; i++) {
    dotProduct += vecA[i] * vecB[i];
    magnitudeA += vecA[i] * vecA[i];
    magnitudeB += vecB[i] * vecB[i];
  }

  magnitudeA = Math.sqrt(magnitudeA);
  magnitudeB = Math.sqrt(magnitudeB);

  return magnitudeA > 0 && magnitudeB > 0 ? dotProduct / (magnitudeA * magnitudeB) : 0;
}

// Gera vetor aleatório
function randomVector(size: number): number[] {
  return Array.from({ length: size }, () => Math.random());
}

// Mede tempo de execução em microsegundos
function benchmark<T>(name: string, fn: () => T, iterations: number = 1000): { name: string; avg_us: number; min_us: number; max_us: number } {
  const times: number[] = [];
  
  // Warmup
  for (let i = 0; i < 100; i++) fn();
  
  // Medição
  for (let i = 0; i < iterations; i++) {
    const start = performance.now();
    fn();
    const end = performance.now();
    times.push((end - start) * 1000); // ms -> us
  }
  
  times.sort((a, b) => a - b);
  const avg = times.reduce((a, b) => a + b, 0) / times.length;
  
  return {
    name,
    avg_us: Math.round(avg * 100) / 100,
    min_us: Math.round(times[0] * 100) / 100,
    max_us: Math.round(times[times.length - 1] * 100) / 100
  };
}

// Simula expansão de query por persona
function expandQuery(query: string, personaName: string): string {
  const suffixes: Record<string, string> = {
    "Skeptic": "problems issues failures real experiences",
    "Detail": "specifications technical details comparison",
    "Historical": "history evolution changes over time",
    "Comparative": "vs alternatives comparison pros cons",
    "Temporal": new Date().getFullYear().toString(),
    "Globalizer": "worldwide international global",
    "Reality": "wrong myth debunked evidence against"
  };
  return `${query} ${suffixes[personaName] || ""}`.trim();
}

// Main
async function main() {
  console.log("\n🏎️  TypeScript Benchmark - DeepResearch AI\n");
  console.log("=".repeat(60));
  
  const results: any[] = [];
  
  // 1. Cosine Similarity - vetores pequenos (8 dims)
  const vecA8 = randomVector(8);
  const vecB8 = randomVector(8);
  results.push(benchmark("cosine_8dim", () => cosineSimilarity(vecA8, vecB8), 10000));
  
  // 2. Cosine Similarity - vetores médios (768 dims - embedding)
  const vecA768 = randomVector(768);
  const vecB768 = randomVector(768);
  results.push(benchmark("cosine_768dim", () => cosineSimilarity(vecA768, vecB768), 1000));
  
  // 3. Cosine Similarity - vetores grandes (1536 dims - OpenAI)
  const vecA1536 = randomVector(1536);
  const vecB1536 = randomVector(1536);
  results.push(benchmark("cosine_1536dim", () => cosineSimilarity(vecA1536, vecB1536), 1000));
  
  // 4. Batch cosine - 100 comparações
  const embeddings100 = Array.from({ length: 100 }, () => randomVector(768));
  const queryEmb = randomVector(768);
  results.push(benchmark("batch_100_cosine", () => {
    embeddings100.forEach(emb => cosineSimilarity(queryEmb, emb));
  }, 100));
  
  // 5. Batch cosine - 1000 comparações  
  const embeddings1000 = Array.from({ length: 1000 }, () => randomVector(768));
  results.push(benchmark("batch_1000_cosine", () => {
    embeddings1000.forEach(emb => cosineSimilarity(queryEmb, emb));
  }, 10));
  
  // 6. Query expansion - 7 personas
  const query = "What are the best practices for Rust programming?";
  const personas = ["Skeptic", "Detail", "Historical", "Comparative", "Temporal", "Globalizer", "Reality"];
  results.push(benchmark("expand_7_personas", () => {
    personas.forEach(p => expandQuery(query, p));
  }, 1000));
  
  // 7. String operations - simula processamento
  const longText = "Lorem ipsum ".repeat(1000);
  results.push(benchmark("string_split_1000", () => {
    longText.split(" ");
  }, 1000));
  
  // 8. Array operations - map/filter/reduce
  const numbers = Array.from({ length: 10000 }, (_, i) => i);
  results.push(benchmark("array_ops_10k", () => {
    numbers.map(n => n * 2).filter(n => n % 3 === 0).reduce((a, b) => a + b, 0);
  }, 100));
  
  // Output JSON para o comparador
  console.log("\n📊 Resultados:\n");
  console.log(JSON.stringify({ language: "TypeScript", results }, null, 2));
}

main().catch(console.error);

