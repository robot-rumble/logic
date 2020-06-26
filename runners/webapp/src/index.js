import * as Comlink from 'comlink'
import RawWasiWorker from './wasi.worker.js'
import { bigInt } from 'wasm-feature-detect'

const lowerPromise = (async () => {
  if (await bigInt()) {
    const mod = await import('@wasmer/wasm-transformer/lib/wasm-pack/bundler')
    return mod.lowerI64Imports
  } else {
    return null
  }
})()

const runnerCache = Object.create(null)

const fetchRunner = async (name) => {
  if (name in runnerCache) return runnerCache[name]
  const prom = (async () => {
    const res = await fetch(`/assets/dist/${name}.wasm`)
    let wasm = await res.arrayBuffer()
    const lowerI64Imports = await lowerPromise
    if (lowerI64Imports) {
      wasm = lowerI64Imports(new Uint8Array(wasm))
    }
    return WebAssembly.compile(wasm)
  })()
  runnerCache[name] = prom
  return prom
}

const runnerMap = {
  Python: 'pyrunner',
  Javascript: 'jsrunner',
}
export default async ({ code, lang }) => {
  const langRunner = await fetchRunner(runnerMap[lang])
  const rawWorker = new RawWasiWorker()
  const WasiRunner = Comlink.wrap(rawWorker)
  const runner = await new WasiRunner(langRunner)
  await runner.setup()
  await runner.init(new TextEncoder().encode(code))
  return [runner, rawWorker]
}
