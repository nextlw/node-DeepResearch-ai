# Deploy do node-DeepResearch-ai no Render

Este guia detalha como fazer o deploy do servidor node-DeepResearch-ai no Render para uso com o elaiRoo.

## Pré-requisitos

- Conta no [Render](https://render.com)
- Conta no GitHub com o repositório do node-DeepResearch-ai
- Chave de API da Jina AI

## Passo a Passo

### 1. Preparar o Repositório

Certifique-se de que o repositório tenha:

1. **Dockerfile** (já existe)
2. **package.json** com script start
3. **Configuração de porta** via variável de ambiente

### 2. Criar Novo Web Service no Render

1. Acesse [Render Dashboard](https://dashboard.render.com)
2. Clique em **"New +"** → **"Web Service"**
3. Conecte seu repositório GitHub
4. Selecione o repositório `node-DeepResearch-ai`

### 3. Configurar o Serviço

#### Configurações Básicas:
- **Name**: `elairoo-deep-research-ai`
- **Region**: Escolha a mais próxima (ex: Oregon)
- **Branch**: `main` ou branch desejada
- **Root Directory**: `.` (raiz do repositório)
- **Runtime**: Docker
- **Instance Type**: Free (para começar) ou Starter ($7/mês)

#### Variáveis de Ambiente:

```env
# Obrigatórias
JINA_API_KEY=your-jina-api-key-here
PORT=3000

# Opcionais (para autenticação)
AUTH_SECRET=your-secret-token-here

# Configurações adicionais
NODE_ENV=production
LOG_LEVEL=info
```

### 4. Configurar Health Check

No Render, configure:
- **Health Check Path**: `/health`
- **Port**: 3000

### 5. Deploy

1. Clique em **"Create Web Service"**
2. Aguarde o build e deploy (pode levar alguns minutos)
3. Copie a URL do serviço (ex: `https://elairoo-deep-research-ai.onrender.com`)

## Configuração no elaiRoo

Adicione as seguintes variáveis de ambiente no elaiRoo:

```env
# .env.local ou configurações do VS Code
AI_DEEP_RESEARCH_SERVER_URL=https://elairoo-deep-research-ai.onrender.com
AI_DEEP_RESEARCH_AUTH_TOKEN=your-secret-token-here
AI_DEEP_RESEARCH_MODEL=jina-deepsearch-v2
```

## Testando a Conexão

### 1. Teste de Health Check
```bash
curl https://elairoo-deep-research-ai.onrender.com/health
# Resposta esperada: {"status":"ok"}
```

### 2. Teste de Modelos
```bash
curl https://elairoo-deep-research-ai.onrender.com/v1/models \
  -H "Authorization: Bearer your-secret-token-here"
```

### 3. Teste de Chat Completion
```bash
curl https://elairoo-deep-research-ai.onrender.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-secret-token-here" \
  -d '{
    "model": "jina-deepsearch-v2",
    "messages": [{"role": "user", "content": "test query"}],
    "stream": false
  }'
```

## Monitoramento

### Logs
- Acesse o dashboard do Render
- Navegue até o serviço
- Clique em "Logs" para ver logs em tempo real

### Métricas
- CPU e memória são mostradas no dashboard
- Configure alertas se necessário

## Troubleshooting

### Problema: Serviço não inicia
- Verifique os logs no Render
- Confirme que todas as variáveis de ambiente estão configuradas
- Verifique se o Dockerfile está correto

### Problema: Timeout nas requisições
- O plano Free do Render pode ter cold starts
- Considere upgrade para Starter para melhor performance
- Adicione retry logic no cliente

### Problema: CORS errors
- Verifique se o servidor está configurado com CORS apropriado
- Headers necessários para SSE:
  ```javascript
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Headers': 'Content-Type, Authorization',
  'Access-Control-Allow-Methods': 'GET, POST, OPTIONS'
  ```

## Configuração Avançada

### Auto-Deploy
1. No Render, ative "Auto-Deploy" para deploy automático em push
2. Configure branch específica se necessário

### Domínio Customizado
1. Adicione domínio customizado nas configurações
2. Configure DNS conforme instruções do Render

### Scaling
- Para produção, considere:
  - Instance type: Standard ou superior
  - Multiple instances com load balancing
  - Redis para cache (adicionar como serviço separado)

## Segurança

1. **Sempre use HTTPS** (Render fornece automaticamente)
2. **Configure AUTH_SECRET** para proteger endpoints
3. **Não exponha JINA_API_KEY** nos logs
4. **Monitore uso** para detectar abusos

## Custos Estimados

- **Free Tier**: 750 horas/mês (suficiente para desenvolvimento)
- **Starter**: $7/mês (recomendado para produção)
- **Standard**: $25/mês (para maior performance)

## Próximos Passos

1. Deploy inicial com configuração básica
2. Teste integração com elaiRoo
3. Monitore performance e logs
4. Ajuste configurações conforme necessário
5. Considere adicionar cache Redis para melhor performance