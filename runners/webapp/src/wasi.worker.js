import { WASI } from '@wasmer/wasi'
import { WasmFs } from '@wasmer/wasmfs'
import wasiBindings from '@wasmer/wasi/lib/bindings/browser'
import * as Comlink from 'comlink'

class WasiRunner {
  constructor(module, files) {
    this.wasmModule = module
    const wasmFs = (this.wasmFs = new WasmFs())
    wasmFs.fromJSON({
      '/dev/stdin': '',
      '/dev/stdout': '',
      '/dev/stderr': '',
      ...files,
    })
    const wasi = (this.wasi = new WASI({
      preopens: { '/': '/' },
      bindings: {
        ...wasiBindings,
        fs: wasmFs.fs,
      },
    }))
    this.wasmImports = wasi.getImports(module)
    this.wasmExports = null
    this._initResult = null
  }
  async setup() {
    const { exports } = await WebAssembly.instantiate(
      this.wasmModule,
      this.wasmImports,
    )
    this.wasmExports = exports
    this.wasi.setMemory(exports.memory)
  }
  get init_result() {
    const { _initResult } = this
    this._initResult = null
    return _initResult
  }
  set_input(input) {
    const ptr = this.wasmExports.__rr_prealloc(input.byteLength)
    const buf = new Uint8Array(
      this.wasmExports.memory.buffer,
      ptr,
      input.byteLength,
    )
    buf.set(input)
  }
  get_output(len) {
    const ptr = this.wasmExports.__rr_io_addr()
    const output = new Uint8Array(this.wasmExports.memory.buffer, ptr, len)
    return output
  }
  init(input) {
    this.set_input(input)
    try {
      const len = this.wasmExports.__rr_init()
      this._initResult = this.get_output(len)
    } catch (e) {
      console.error('error while initializing', e, e && e.stack)
      console.error(this.wasmFs.fs.readFileSync('/dev/stderr', 'utf8'))
      return {
        output: new TextEncoder().encode('{"Err":{"InternalError":null}}'),
      }
    }
  }
  run_turn(input) {
    const { fs } = this.wasmFs
    fs.writeFileSync('/dev/stdout', '')
    this.set_input(input)
    let logs = ''
    try {
      const len = this.wasmExports.__rr_run_turn()
      logs = fs.readFileSync('/dev/stdout', 'utf8')
      return { output: this.get_output(len), logs }
    } catch (e) {
      console.error('error while running turn', e, e && e.stack)
      console.error(fs.readFileSync('/dev/stderr', 'utf8'))
      return {
        output: new TextEncoder().encode(
          '{"robot_outputs":{"Err":{"InternalError":null}}}',
        ),
        logs,
      }
    }
  }
}

Comlink.expose(WasiRunner)
