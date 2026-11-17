/** Shared Tailwind class fragments so every page uses the same controls. */

export const card = 'rounded-xl border border-slate-200 bg-white shadow-sm'

export const label = 'block text-sm font-medium text-slate-700'

export const input =
  'w-full rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm text-slate-900 shadow-sm outline-none focus:border-blue-500 focus:ring-2 focus:ring-blue-500/20'

export const button =
  'inline-flex items-center justify-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50'

export const primaryButton = `${button} bg-slate-900 text-white hover:bg-slate-800`

export const secondaryButton = `${button} border border-slate-300 bg-white text-slate-700 hover:bg-slate-50`

export const link = 'text-blue-600 hover:text-blue-800 hover:underline'

export const codeBox =
  'rounded-lg bg-slate-100 px-3 py-2 font-mono text-xs text-slate-700'
