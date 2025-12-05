import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import {
  XmpFile,
  XmpMeta,
  ReadOptions,
  Namespace,
  namespace_uri,
  register_namespace,
  is_namespace_registered,
  get_all_registered_namespaces,
  get_builtin_namespace_uris
} from '../utils/wasm'

export function useXmp() {
  const { t } = useI18n()
  const xmpFile = ref<XmpFile | null>(null)
  const currentFileData = ref<Uint8Array | null>(null)
  const originalFileData = ref<Uint8Array | null>(null) // Store original file data for revert
  const currentFileName = ref<string>('')
  const xmpProperties = ref<Array<{ label: string; value: string; extended?: boolean }>>([])
  const xmpPacket = ref('')
  const registeredNamespaces = ref<Array<{ uri: string; prefix: string }>>([])

  const showMessage = (key: string, type: 'success' | 'error' | 'info' | 'warning' = 'info') => {
    ElMessage({
      message: t(`messages.${key}`),
      type,
      duration: 3000
    })
  }

  const builtinNamespaceUris = ref<string[]>([])

  const updateRegisteredNamespaces = () => {
    try {
      // Get built-in namespace URIs from Rust
      builtinNamespaceUris.value = get_builtin_namespace_uris()
      
      const allNamespaces = get_all_registered_namespaces()
      console.log('get_all_registered_namespaces result:', allNamespaces, typeof allNamespaces)
      
      // get_all_registered_namespaces returns a JS object with URI as key and prefix as value
      if (allNamespaces && typeof allNamespaces === 'object') {
        const entries = Object.entries(allNamespaces)
        console.log('Namespace entries:', entries)
        registeredNamespaces.value = entries
          .map(([uri, prefix]) => ({ uri, prefix: prefix as string }))
          .sort((a, b) => a.prefix.localeCompare(b.prefix))
        console.log('Final registeredNamespaces:', registeredNamespaces.value)
      } else {
        console.warn('get_all_registered_namespaces returned invalid format:', allNamespaces)
        registeredNamespaces.value = []
      }
    } catch (error) {
      console.error('get_all_registered_namespaces error:', error)
      registeredNamespaces.value = []
    }
  }

  const readXmp = () => {
    if (!xmpFile.value) {
      showMessage('uploadFileFirst', 'error')
      return
    }

    try {
      const meta = xmpFile.value.get_xmp()
      if (!meta) {
        xmpProperties.value = []
        xmpPacket.value = ''
        showMessage('noXmpData', 'info')
        return
      }

      const commonProperties = [
        { ns: namespace_uri(Namespace.Xmp), name: 'CreatorTool', label: 'CreatorTool' },
        { ns: namespace_uri(Namespace.Xmp), name: 'CreateDate', label: 'CreateDate' },
        { ns: namespace_uri(Namespace.Xmp), name: 'ModifyDate', label: 'ModifyDate' },
        { ns: namespace_uri(Namespace.Dc), name: 'title', label: 'Title' },
        { ns: namespace_uri(Namespace.Dc), name: 'creator', label: 'Creator' },
        { ns: namespace_uri(Namespace.Dc), name: 'description', label: 'Description' },
      ]

      const properties: Array<{ label: string; value: string; extended?: boolean }> = []
      commonProperties.forEach(prop => {
        try {
          const value = meta.get_property(prop.ns, prop.name)
          if (value !== undefined && value !== null) {
            properties.push({ ...prop, value: value as string })
          }
        } catch {
          // Property not found or error reading, skip
        }
      })

      const extendedProperties = [
        { ns: 'http://www.tc260.org.cn/ns/AIGC/1.0/', name: 'AIGC', label: 'AIGC (AI Generated Content)' },
        { ns: 'http://www.tc260.org.cn/ns/AIGC/1.0/', name: 'AIGCType', label: 'AIGC Type' },
        { ns: 'http://www.tc260.org.cn/ns/AIGC/1.0/', name: 'AIGCModel', label: 'AIGC Model' },
      ]

      extendedProperties.forEach(prop => {
        try {
          if (is_namespace_registered(prop.ns)) {
            const value = meta.get_property(prop.ns, prop.name)
            if (value !== undefined && value !== null) {
              properties.push({ ...prop, value: value as string, extended: true })
            }
          }
        } catch {
          // ignore
        }
      })

      xmpProperties.value = properties

      try {
        xmpPacket.value = meta.serialize_packet() || ''
      } catch (e) {
        console.error('Failed to serialize XMP packet:', e)
        xmpPacket.value = ''
      }

      showMessage('xmpRead', 'success')
    } catch (error) {
      showMessage('readFailed', 'error')
      console.error(error)
    }
  }

  const loadFile = async (file: File) => {
    return new Promise<{ fileData: Uint8Array; fileName: string; xmpFile: XmpFile } | null>((resolve) => {
      const reader = new FileReader()
      reader.onload = (e) => {
        try {
          if (!e.target?.result) return
          const fileData = new Uint8Array(e.target.result as ArrayBuffer)
          const xmpFileInstance = new XmpFile()

          let loadSuccess = false
          try {
            const options1 = new ReadOptions()
            options1.for_update() // Required for write_to_bytes() later
            options1.use_smart_handler()
            options1.only_xmp()
            xmpFileInstance.from_bytes_with(fileData, options1)
            loadSuccess = true
          } catch (error) {
            console.log('Smart handler failed, trying packet scanning...', error)
            try {
              const options2 = new ReadOptions()
              options2.for_update() // Required for write_to_bytes() later
              options2.use_packet_scanning()
              xmpFileInstance.from_bytes_with(fileData, options2)
              loadSuccess = true
            } catch {
              showMessage('loadFailed', 'error')
              resolve(null)
              return
            }
          }

          if (loadSuccess) {
            currentFileData.value = fileData
            originalFileData.value = fileData.slice() // Store a copy of original data
            currentFileName.value = file.name
            xmpFile.value = xmpFileInstance
            showMessage('fileLoaded', 'success')
            readXmp()
            updateRegisteredNamespaces()
            resolve({ fileData, fileName: file.name, xmpFile: xmpFileInstance })
          }
        } catch (error) {
          showMessage('loadFailed', 'error')
          console.error('Unexpected error:', error)
          resolve(null)
        }
      }
      reader.readAsArrayBuffer(file)
    })
  }

  const setProperty = (namespace: string, property: string, value: string) => {
    if (!xmpFile.value) {
      showMessage('uploadFileFirst', 'error')
      return
    }

    if (!namespace || !property || !value) {
      showMessage('fillCompleteInfo', 'error')
      return
    }

    try {
      let meta = xmpFile.value.get_xmp()
      if (!meta) {
        meta = new XmpMeta()
      }

      try {
        meta.set_property(namespace, property, value)
        xmpFile.value.put_xmp(meta)
        showMessage('propertySet', 'success')
        readXmp()
      } catch (error) {
        showMessage('setFailed', 'error')
        console.error('set_property error:', error)
      }
    } catch (error) {
      showMessage('setFailed', 'error')
      console.error('Unexpected error:', error)
    }
  }

  const deleteProperty = (namespace: string, property: string) => {
    if (!xmpFile.value) {
      showMessage('uploadFileFirst', 'error')
      return
    }

    if (!namespace || !property) {
      showMessage('fillNamespaceAndProperty', 'error')
      return
    }

    try {
      const meta = xmpFile.value.get_xmp()
      if (meta) {
        try {
          meta.delete_property(namespace, property)
          xmpFile.value.put_xmp(meta)
          showMessage('propertyDeleted', 'success')
          readXmp()
        } catch (error) {
          showMessage('deleteFailed', 'error')
          console.error('delete_property error:', error)
        }
      } else {
        showMessage('noXmpData', 'info')
      }
    } catch (error) {
      showMessage('deleteFailed', 'error')
      console.error('Unexpected error:', error)
    }
  }

  const registerNamespace = (uri: string, prefix: string) => {
    if (!uri || !prefix) {
      showMessage('fillNamespaceUriAndPrefix', 'error')
      return
    }

    try {
      register_namespace(uri, prefix)
      showMessage('namespaceRegistered', 'success')
      updateRegisteredNamespaces()
      if (xmpFile.value) {
        readXmp()
      }
    } catch (error) {
      showMessage('registerFailed', 'error')
      console.error('register_namespace error:', error)
    }
  }

  const downloadModifiedFile = () => {
    if (!xmpFile.value || !currentFileData.value) {
      showMessage('uploadFileFirst', 'error')
      return
    }

    try {
      // write_to_bytes returns Result<Vec<u8>, XmpError> in Rust
      // In JavaScript, wasm-bindgen converts Result to: success returns value, error throws exception
      let modifiedData: Uint8Array
      try {
        const result = xmpFile.value.write_to_bytes()
        if (!result || result.length === 0) {
          showMessage('noData', 'error')
          return
        }
        // Convert Vec<u8> (which becomes Uint8Array in JS) to Uint8Array if needed
        modifiedData = result instanceof Uint8Array ? result : new Uint8Array(result)
      } catch (error: unknown) {
        console.error('write_to_bytes error:', error)
        // Extract error message if available
        let errorMessage = 'Unknown error'
        if (error && typeof error === 'object') {
          const err = error as { message?: string; kind?: string; toString?: () => string }
          if (err.message) {
            errorMessage = err.message
          } else if (err.toString && err.toString() !== '[object Object]') {
            errorMessage = err.toString()
          } else if (err.kind) {
            errorMessage = `Error kind: ${err.kind}`
          }
        }
        console.error('Error details:', errorMessage)
        ElMessage({
          message: `${t('messages.saveFailed')}: ${errorMessage}`,
          type: 'error',
          duration: 5000
        })
        return
      }

      if (!modifiedData || modifiedData.length === 0) {
        showMessage('noData', 'error')
        return
      }

      // Get file extension from original filename to preserve it
      const fileName = currentFileName.value || 'modified_file'
      const fileExtension = fileName.includes('.') ? fileName.substring(fileName.lastIndexOf('.')) : ''
      const downloadFileName = fileName.includes('.') 
        ? fileName.substring(0, fileName.lastIndexOf('.')) + '_modified' + fileExtension
        : fileName + '_modified'

      // Convert Uint8Array to ArrayBuffer for Blob constructor
      // Use slice() to ensure we have a proper ArrayBuffer
      const blob = new Blob([modifiedData.slice()], { type: 'application/octet-stream' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = downloadFileName
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)

      showMessage('fileSaved', 'success')
    } catch (error) {
      showMessage('saveFailed', 'error')
      console.error('downloadModifiedFile error:', error)
    }
  }

  const revertToOriginal = () => {
    if (!xmpFile.value || !originalFileData.value) {
      showMessage('uploadFileFirst', 'error')
      return
    }

    try {
      // Reload the original file data
      const xmpFileInstance = new XmpFile()
      let loadSuccess = false

      try {
        const options1 = new ReadOptions()
        options1.for_update() // Required for write_to_bytes() later
        options1.use_smart_handler()
        options1.only_xmp()
        xmpFileInstance.from_bytes_with(originalFileData.value, options1)
        loadSuccess = true
      } catch {
        try {
          const options2 = new ReadOptions()
          options2.for_update() // Required for write_to_bytes() later
          options2.use_packet_scanning()
          xmpFileInstance.from_bytes_with(originalFileData.value, options2)
          loadSuccess = true
        } catch {
          showMessage('loadFailed', 'error')
          return
        }
      }

      if (loadSuccess) {
        currentFileData.value = originalFileData.value.slice()
        xmpFile.value = xmpFileInstance
        readXmp()
        showMessage('fileReverted', 'success')
      }
    } catch (error) {
      showMessage('loadFailed', 'error')
      console.error('revert error:', error)
    }
  }

  const reset = () => {
    xmpFile.value = null
    currentFileData.value = null
    originalFileData.value = null
    currentFileName.value = ''
    xmpProperties.value = []
    xmpPacket.value = ''
    showMessage('resetComplete', 'info')
  }

  const getPropertyValue = (namespace: string, property: string): string | null => {
    if (!xmpFile.value) {
      return null
    }
    try {
      const meta = xmpFile.value.get_xmp()
      if (!meta) {
        return null
      }
      const value = meta.get_property(namespace, property)
      if (value !== undefined && value !== null) {
        return value as string
      }
      return null
    } catch {
      return null
    }
  }

  return {
    xmpFile,
    currentFileData,
    currentFileName,
    xmpProperties,
    xmpPacket,
    registeredNamespaces,
    builtinNamespaceUris,
    loadFile,
    readXmp,
    setProperty,
    deleteProperty,
    registerNamespace,
    downloadModifiedFile,
    revertToOriginal,
    reset,
    updateRegisteredNamespaces,
    getPropertyValue,
  }
}

