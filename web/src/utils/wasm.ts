import init, {
  XmpFile,
  XmpMeta,
  ReadOptions,
  Namespace,
  namespace_uri,
  register_namespace,
  get_namespace_prefix,
  get_namespace_uri,
  is_namespace_registered,
  get_all_registered_namespaces,
  get_builtin_namespace_uris
} from '../../pkg/xmpkit.js'

let wasmInitialized = false

export async function initWasm() {
  if (!wasmInitialized) {
    await init()
    wasmInitialized = true
    
    // Register AIGC namespace by default
    try {
      register_namespace('http://www.tc260.org.cn/ns/AIGC/1.0/', 'TC260')
    } catch (error) {
      // Namespace might already be registered, ignore error
      console.log('AIGC namespace registration:', error)
    }
  }
  return true
}

export {
  XmpFile,
  XmpMeta,
  ReadOptions,
  Namespace,
  namespace_uri,
  register_namespace,
  get_namespace_prefix,
  get_namespace_uri,
  is_namespace_registered,
  get_all_registered_namespaces,
  get_builtin_namespace_uris
}

