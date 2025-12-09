/**
 * 🕐 Step Timing Benchmark - DeepResearch AI
 *
 * Mede o tempo de cada step/ação do agente durante uma pesquisa.
 *
 * Uso:
 *   npx ts-node benchmark/step-timing.ts "Sua pergunta aqui"
 */

import { getResponse } from '../src/agent'
import { StepAction } from '../src/types'
import { ActionTracker } from '../src/utils/action-tracker'
import { TokenTracker } from '../src/utils/token-tracker'

interface StepTiming {
  step: number
  action: string
  think: string
  startTime: number
  endTime?: number
  durationMs?: number
}

interface BenchmarkResult {
  question: string
  totalDurationMs: number
  totalSteps: number
  steps: StepTiming[]
  actionSummary: Record<string, { count: number; totalMs: number; avgMs: number }>
  finalAction: string
}

// Cores para o terminal
const colors = {
  reset: '\x1b[0m',
  bright: '\x1b[1m',
  dim: '\x1b[2m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m',
  red: '\x1b[31m',
}

function getActionColor(action: string): string {
  const actionColors: Record<string, string> = {
    search: colors.blue,
    answer: colors.green,
    reflect: colors.magenta,
    visit: colors.cyan,
    coding: colors.yellow,
  }
  return actionColors[action] || colors.reset
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms.toFixed(0)}ms`
  if (ms < 60000) return `${(ms / 1000).toFixed(2)}s`
  return `${(ms / 60000).toFixed(2)}min`
}

async function benchmarkSteps(question: string): Promise<BenchmarkResult> {
  console.log(`\n${colors.bright}🔬 Step Timing Benchmark${colors.reset}`)
  console.log(`${'─'.repeat(60)}`)
  console.log(`${colors.dim}Pergunta:${colors.reset} ${question}\n`)

  const steps: StepTiming[] = []
  let currentStep: StepTiming | null = null
  let stepCounter = 0
  const startTime = performance.now()

  // Criar trackers
  const tokenTracker = new TokenTracker(1_000_000)
  const actionTracker = new ActionTracker()

  // Escutar eventos de ação
  actionTracker.on('action', (thisStep: StepAction) => {
    const now = performance.now()

    // Finalizar step anterior
    if (currentStep && !currentStep.endTime) {
      currentStep.endTime = now
      currentStep.durationMs = currentStep.endTime - currentStep.startTime

      // Log do step finalizado
      const color = getActionColor(currentStep.action)
      console.log(
        `${colors.dim}Step ${currentStep.step}${colors.reset} │ ` +
        `${color}${currentStep.action.toUpperCase().padEnd(7)}${colors.reset} │ ` +
        `${formatDuration(currentStep.durationMs).padStart(8)} │ ` +
        `${colors.dim}${currentStep.think.substring(0, 50)}...${colors.reset}`
      )
    }

    // Iniciar novo step
    stepCounter++
    currentStep = {
      step: stepCounter,
      action: thisStep.action,
      think: thisStep.think || '',
      startTime: now,
    }
    steps.push(currentStep)
  })

  console.log(`${colors.dim}Step    │ Ação    │ Duração  │ Pensamento${colors.reset}`)
  console.log(`${'─'.repeat(60)}`)

  // Executar o agente
  try {
    const result = await getResponse(
      question,
      500_000, // Token budget reduzido para teste
      2,
      { tokenTracker, actionTracker },
      undefined,
      50, // URLs reduzidas
      false,
      [],
      [],
      [],
      5,
      0.8,
      'pt-BR',
    )

    // Finalizar último step
    const endTime = performance.now()
    if (currentStep && !currentStep.endTime) {
      currentStep.endTime = endTime
      currentStep.durationMs = currentStep.endTime - currentStep.startTime

      const color = getActionColor(currentStep.action)
      console.log(
        `${colors.dim}Step ${currentStep.step}${colors.reset} │ ` +
        `${color}${currentStep.action.toUpperCase().padEnd(7)}${colors.reset} │ ` +
        `${formatDuration(currentStep.durationMs).padStart(8)} │ ` +
        `${colors.dim}${currentStep.think.substring(0, 50)}...${colors.reset}`
      )
    }

    const totalDuration = endTime - startTime

    // Calcular sumário por ação
    const actionSummary: Record<string, { count: number; totalMs: number; avgMs: number }> = {}

    for (const step of steps) {
      if (!step.durationMs) continue

      if (!actionSummary[step.action]) {
        actionSummary[step.action] = { count: 0, totalMs: 0, avgMs: 0 }
      }
      actionSummary[step.action].count++
      actionSummary[step.action].totalMs += step.durationMs
    }

    // Calcular médias
    for (const action of Object.keys(actionSummary)) {
      actionSummary[action].avgMs = actionSummary[action].totalMs / actionSummary[action].count
    }

    return {
      question,
      totalDurationMs: totalDuration,
      totalSteps: steps.length,
      steps,
      actionSummary,
      finalAction: result.result.action,
    }
  } catch (error) {
    console.error(`${colors.red}Erro:${colors.reset}`, error)
    throw error
  }
}

function printSummary(result: BenchmarkResult) {
  console.log(`\n${'─'.repeat(60)}`)
  console.log(`${colors.bright}📊 Resumo${colors.reset}\n`)

  console.log(`${colors.dim}Tempo total:${colors.reset} ${formatDuration(result.totalDurationMs)}`)
  console.log(`${colors.dim}Total de steps:${colors.reset} ${result.totalSteps}`)
  console.log(`${colors.dim}Ação final:${colors.reset} ${result.finalAction}`)

  console.log(`\n${colors.bright}⏱️  Tempo por Ação${colors.reset}\n`)
  console.log(`${'Ação'.padEnd(10)} │ ${'Qtd'.padStart(4)} │ ${'Total'.padStart(10)} │ ${'Média'.padStart(10)}`)
  console.log(`${'─'.repeat(45)}`)

  const sortedActions = Object.entries(result.actionSummary)
    .sort((a, b) => b[1].totalMs - a[1].totalMs)

  for (const [action, stats] of sortedActions) {
    const color = getActionColor(action)
    console.log(
      `${color}${action.padEnd(10)}${colors.reset} │ ` +
      `${stats.count.toString().padStart(4)} │ ` +
      `${formatDuration(stats.totalMs).padStart(10)} │ ` +
      `${formatDuration(stats.avgMs).padStart(10)}`
    )
  }

  // Timeline visual
  console.log(`\n${colors.bright}📈 Timeline dos Steps${colors.reset}\n`)

  const maxDuration = Math.max(...result.steps.map(s => s.durationMs || 0))
  const barWidth = 40

  for (const step of result.steps) {
    if (!step.durationMs) continue

    const barLength = Math.round((step.durationMs / maxDuration) * barWidth)
    const bar = '█'.repeat(barLength) + '░'.repeat(barWidth - barLength)
    const color = getActionColor(step.action)

    console.log(
      `${colors.dim}${step.step.toString().padStart(2)}${colors.reset} ` +
      `${color}${step.action.substring(0, 3).toUpperCase()}${colors.reset} ` +
      `${color}${bar}${colors.reset} ` +
      `${formatDuration(step.durationMs)}`
    )
  }

  // JSON para análise
  console.log(`\n${colors.bright}📋 JSON (para análise)${colors.reset}\n`)
  console.log(JSON.stringify({
    question: result.question,
    totalDurationMs: Math.round(result.totalDurationMs),
    totalSteps: result.totalSteps,
    finalAction: result.finalAction,
    actionSummary: Object.fromEntries(
      Object.entries(result.actionSummary).map(([k, v]) => [
        k,
        { count: v.count, totalMs: Math.round(v.totalMs), avgMs: Math.round(v.avgMs) }
      ])
    ),
    steps: result.steps.map(s => ({
      step: s.step,
      action: s.action,
      durationMs: Math.round(s.durationMs || 0),
    }))
  }, null, 2))
}

// Main
async function main() {
  const question = process.argv[2] || 'Quais são as melhores práticas de programação em TypeScript em 2024?'

  try {
    const result = await benchmarkSteps(question)
    printSummary(result)
  } catch (error) {
    console.error(`\n${colors.red}Falha no benchmark:${colors.reset}`, error)
    process.exit(1)
  }
}

main()
