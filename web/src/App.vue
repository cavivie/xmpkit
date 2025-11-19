<template>
  <el-container class="app-container">
    <el-header>
      <div class="header-content">
        <div class="title-with-logo">
          <img :src="logoPath" alt="XMPKit" class="header-logo" />
          <h1>{{ $t('common.title') }}</h1>
        </div>
        <div class="header-actions">
          <el-tooltip 
            :content="localeTooltip" 
            placement="bottom"
            :trigger="isMobile ? 'manual' : 'hover'"
            v-model:visible="localeTooltipVisible"
            :hide-after="0"
            :show-after="0"
          >
            <el-button
              :icon="localeIcon"
              circle
              @click="handleLocaleClick"
              class="locale-toggle-btn"
            />
          </el-tooltip>
          <el-tooltip 
            :content="themeTooltip" 
            placement="bottom"
            :trigger="isMobile ? 'manual' : 'hover'"
            v-model:visible="themeTooltipVisible"
            :hide-after="0"
            :show-after="0"
          >
            <el-button
              :icon="themeIcon"
              circle
              @click="handleThemeClick"
              class="theme-toggle-btn"
            />
          </el-tooltip>
        </div>
      </div>
    </el-header>
    
    <el-main>
      <!-- File Upload Area -->
      <el-card class="upload-card">
        <el-upload
          ref="uploadRef"
          :auto-upload="false"
          :on-change="handleFileChange"
          :show-file-list="false"
          drag
          accept="image/*,audio/*,video/*"
        >
          <el-icon class="el-icon--upload"><upload-filled /></el-icon>
          <div class="el-upload__text">
            {{ $t('common.upload') }}
          </div>
          <template #tip>
            <div class="el-upload__tip">
              {{ $t('common.uploadTip') }}
            </div>
          </template>
        </el-upload>
      </el-card>

      <!-- File Preview -->
      <el-card v-if="filePreview" class="preview-card">
        <template #header>
          <span>{{ $t('common.filePreview') }}</span>
        </template>
        <div class="preview-container">
          <img v-if="filePreview.type.startsWith('image/')" :src="filePreview.url" :alt="filePreview.name" />
          <video v-else-if="filePreview.type.startsWith('video/')" :src="filePreview.url" controls />
          <audio v-else-if="filePreview.type.startsWith('audio/')" :src="filePreview.url" controls />
          <div v-else class="file-placeholder">
            <p>📄 {{ filePreview.name }}</p>
            <p class="file-placeholder-tip">{{ $t('common.noPreview') }}</p>
          </div>
        </div>
        <el-descriptions :column="1" border style="margin-top: 20px;">
          <el-descriptions-item :label="$t('common.fileName')">{{ filePreview.name }}</el-descriptions-item>
          <el-descriptions-item :label="$t('common.fileSize')">{{ filePreview.size }} KB</el-descriptions-item>
          <el-descriptions-item :label="$t('common.fileType')">{{ filePreview.type }}</el-descriptions-item>
        </el-descriptions>
      </el-card>

      <!-- Operation Buttons -->
      <el-card v-if="xmpFile" class="controls-card">
        <template #header>
          <span>{{ $t('common.operations') }}</span>
        </template>
        <el-button type="primary" @click="readXmp">
          <el-icon><document /></el-icon>
          {{ $t('common.readXmp') }}
        </el-button>
        <el-button type="success" @click="downloadModifiedFile">
          <el-icon><download /></el-icon>
          {{ $t('common.downloadModified') }}
        </el-button>
        <el-button type="info" @click="revertToOriginal">
          <el-icon><refresh-left /></el-icon>
          {{ $t('common.revert') }}
        </el-button>
        <el-button type="warning" @click="reset">
          <el-icon><delete /></el-icon>
          {{ $t('common.reset') }}
        </el-button>
      </el-card>

      <!-- XMP Properties Display -->
      <el-card v-if="xmpFile" class="xmp-card">
        <template #header>
          <span>{{ $t('common.xmpProperties') }}</span>
        </template>
        <el-empty v-if="!xmpProperties.length" :description="$t('common.noProperties')" />
        <el-descriptions v-else :column="1" border>
          <el-descriptions-item
            v-for="prop in xmpProperties"
            :key="prop.label"
            :label="prop.label"
          >
            {{ prop.value }}
          </el-descriptions-item>
        </el-descriptions>
      </el-card>

      <!-- Namespace Management -->
      <el-collapse v-model="namespaceCollapseActiveNames" class="namespace-card">
        <el-collapse-item name="namespace-management" :title="$t('common.namespaceManagement')">
          <el-form :inline="true" @submit.prevent="handleRegisterNamespace" class="namespace-form">
          <el-form-item :label="$t('common.namespaceUri')">
            <el-input
              v-model="namespaceForm.uri"
              :placeholder="$t('common.namespaceUri')"
              class="namespace-uri-input"
            />
          </el-form-item>
          <el-form-item :label="$t('common.namespacePrefix')">
            <el-input
              v-model="namespaceForm.prefix"
              :placeholder="$t('common.namespacePrefix')"
              class="namespace-prefix-input"
            />
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="handleRegisterNamespace">
              <el-icon><plus /></el-icon>
              {{ $t('common.registerNamespace') }}
            </el-button>
          </el-form-item>
        </el-form>
        
        <el-divider />
        
        <div class="namespace-table-header">
          <span>{{ $t('common.registeredNamespaces') }} ({{ registeredNamespaces.length }})</span>
        </div>
        <el-table :data="registeredNamespaces" stripe style="width: 100%">
          <el-table-column prop="prefix" :label="$t('common.namespacePrefix')" width="150">
            <template #default="{ row }">
              <el-tag type="primary">{{ row.prefix }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="uri" :label="$t('common.namespaceUri')">
            <template #default="{ row }">
              <code class="namespace-uri">{{ row.uri }}</code>
            </template>
          </el-table-column>
        </el-table>
        </el-collapse-item>
      </el-collapse>

      <!-- Edit Properties -->
      <el-card v-if="xmpFile" class="edit-card">
        <template #header>
          <span>{{ $t('common.editProperties') }}</span>
        </template>
        <el-form :model="propertyForm" label-width="120px">
          <el-form-item :label="$t('common.namespaceUri')">
            <el-select
              v-model="propertyForm.namespace"
              :placeholder="$t('common.namespaceUri')"
              filterable
              allow-create
              style="width: 100%"
            >
              <el-option-group
                v-if="customNamespaces.length > 0"
                :label="$t('common.customNamespaces')"
              >
                <el-option
                  v-for="ns in customNamespaces"
                  :key="ns.uri"
                  :label="`${ns.prefix} (${ns.uri})`"
                  :value="ns.uri"
                >
                  <span style="float: left">{{ ns.prefix }}</span>
                  <span style="float: right; color: #8492a6; font-size: 13px">{{ ns.uri }}</span>
                </el-option>
              </el-option-group>
              <el-option-group
                v-if="builtinNamespaces.length > 0"
                :label="$t('common.builtinNamespaces')"
              >
                <el-option
                  v-for="ns in builtinNamespaces"
                  :key="ns.uri"
                  :label="`${ns.prefix} (${ns.uri})`"
                  :value="ns.uri"
                >
                  <span style="float: left">{{ ns.prefix }}</span>
                  <span style="float: right; color: #8492a6; font-size: 13px">{{ ns.uri }}</span>
                </el-option>
              </el-option-group>
            </el-select>
          </el-form-item>
          <el-form-item :label="$t('common.propertyName')">
            <template #label>
              <span>{{ $t('common.propertyName') }}</span>
              <el-tooltip :content="$t('common.propertyNameTip')" placement="top">
                <el-icon style="margin-left: 4px; color: #909399; cursor: help;"><question-filled /></el-icon>
              </el-tooltip>
            </template>
            <el-select
              v-model="propertyForm.property"
              :placeholder="$t('common.propertyName')"
              filterable
              allow-create
              style="width: 100%"
            >
              <el-option
                v-for="prop in commonPropertyNames"
                :key="prop.name"
                :label="prop.label"
                :value="prop.name"
              >
                <span style="float: left">{{ prop.label }}</span>
                <span style="float: right; color: #8492a6; font-size: 13px">{{ prop.name }}</span>
              </el-option>
            </el-select>
            <div v-if="commonPropertyNames.length > 0" style="margin-top: 4px; font-size: 12px; color: #909399;">
              {{ $t('common.availableProperties', { count: commonPropertyNames.length }) }}
            </div>
            <div v-else style="margin-top: 4px; font-size: 12px; color: #909399;">
              {{ $t('common.noCommonProperties') }}
            </div>
          </el-form-item>
          <el-form-item :label="$t('common.propertyValue')">
            <template #label>
              <span>{{ $t('common.propertyValue') }}</span>
              <el-tooltip :content="$t('common.propertyValueTip')" placement="top">
                <el-icon style="margin-left: 4px; color: #909399; cursor: help;"><question-filled /></el-icon>
              </el-tooltip>
            </template>
            <el-input
              v-model="propertyForm.value"
              type="textarea"
              :rows="3"
              :placeholder="$t('common.propertyValue')"
            />
          </el-form-item>
          <el-form-item class="edit-buttons">
            <el-button type="primary" @click="handleSetProperty">
              <el-icon><check /></el-icon>
              {{ $t('common.setProperty') }}
            </el-button>
            <el-button type="danger" @click="handleDeleteProperty">
              <el-icon><delete /></el-icon>
              {{ $t('common.deleteProperty') }}
            </el-button>
          </el-form-item>
        </el-form>
      </el-card>

      <!-- Raw XMP Packet -->
      <el-card v-if="xmpPacket" class="packet-card">
        <template #header>
          <span>{{ $t('common.rawXmpPacket') }}</span>
        </template>
        <el-scrollbar height="400px">
          <pre class="xmp-packet">{{ xmpPacket }}</pre>
        </el-scrollbar>
      </el-card>
    </el-main>
  </el-container>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch, onMounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  UploadFilled,
  Document,
  Download,
  RefreshLeft,
  Delete,
  Plus,
  Check,
  QuestionFilled,
  Sunny,
  Moon,
  Monitor,
  Switch
} from '@element-plus/icons-vue'
import { useXmp } from './composables/useXmp'
import { initWasm } from './utils/wasm'

const { locale, t } = useI18n()
const uploadRef = ref()
const namespaceCollapseActiveNames = ref<string[]>([]) // Empty array means collapsed by default
const theme = ref<'auto' | 'light' | 'dark'>('auto')

// Detect mobile device
const isMobile = ref(false)
const localeTooltipVisible = ref(false)
const themeTooltipVisible = ref(false)

const updateIsMobile = () => {
  if (typeof window !== 'undefined') {
    isMobile.value = window.innerWidth <= 768
  }
}

const handleLocaleClick = () => {
  toggleLocale()
  if (isMobile.value) {
    localeTooltipVisible.value = true
    setTimeout(() => {
      localeTooltipVisible.value = false
    }, 1000)
  }
}

const handleThemeClick = () => {
  toggleTheme()
  if (isMobile.value) {
    themeTooltipVisible.value = true
    setTimeout(() => {
      themeTooltipVisible.value = false
    }, 1000)
  }
}

const filePreview = ref<{
  name: string
  size: string
  type: string
  url: string
} | null>(null)

const {
  xmpFile,
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
} = useXmp()

const namespaceForm = reactive({
  uri: '',
  prefix: ''
})

const propertyForm = reactive({
  namespace: '',
  property: '',
  value: ''
})

// Separate custom and built-in namespaces
const customNamespaces = computed(() => {
  return registeredNamespaces.value.filter(ns => !builtinNamespaceUris.value.includes(ns.uri))
})

const builtinNamespaces = computed(() => {
  return registeredNamespaces.value.filter(ns => builtinNamespaceUris.value.includes(ns.uri))
})

// Common property names for different namespaces
const commonPropertyNames = computed(() => {
  const namespaceUri = propertyForm.namespace
  
  // XMP Basic namespace
  if (namespaceUri === 'http://ns.adobe.com/xap/1.0/') {
    return [
      { name: 'CreatorTool', label: 'CreatorTool' },
      { name: 'CreateDate', label: 'CreateDate' },
      { name: 'ModifyDate', label: 'ModifyDate' },
      { name: 'MetadataDate', label: 'MetadataDate' },
      { name: 'Identifier', label: 'Identifier' },
      { name: 'Nickname', label: 'Nickname' },
      { name: 'Rating', label: 'Rating' },
      { name: 'Label', label: 'Label' },
    ]
  }
  
  // Dublin Core namespace
  if (namespaceUri === 'http://purl.org/dc/elements/1.1/') {
    return [
      { name: 'title', label: 'Title' },
      { name: 'creator', label: 'Creator' },
      { name: 'subject', label: 'Subject' },
      { name: 'description', label: 'Description' },
      { name: 'publisher', label: 'Publisher' },
      { name: 'contributor', label: 'Contributor' },
      { name: 'date', label: 'Date' },
      { name: 'type', label: 'Type' },
      { name: 'format', label: 'Format' },
      { name: 'identifier', label: 'Identifier' },
      { name: 'source', label: 'Source' },
      { name: 'language', label: 'Language' },
      { name: 'relation', label: 'Relation' },
      { name: 'coverage', label: 'Coverage' },
      { name: 'rights', label: 'Rights' },
    ]
  }
  
  // EXIF namespace
  if (namespaceUri === 'http://ns.adobe.com/exif/1.0/') {
    return [
      { name: 'ExifVersion', label: 'ExifVersion' },
      { name: 'ColorSpace', label: 'ColorSpace' },
      { name: 'PixelXDimension', label: 'PixelXDimension' },
      { name: 'PixelYDimension', label: 'PixelYDimension' },
      { name: 'DateTimeOriginal', label: 'DateTimeOriginal' },
      { name: 'DateTimeDigitized', label: 'DateTimeDigitized' },
      { name: 'ExposureTime', label: 'ExposureTime' },
      { name: 'FNumber', label: 'FNumber' },
      { name: 'ExposureProgram', label: 'ExposureProgram' },
      { name: 'ISOSpeedRatings', label: 'ISOSpeedRatings' },
    ]
  }
  
  // AIGC namespace
  if (namespaceUri === 'http://www.tc260.org.cn/ns/AIGC/1.0/') {
    return [
      { name: 'AIGC', label: 'AIGC (AI Generated Content)' },
    ]
  }
  
  // XMP Rights namespace
  if (namespaceUri === 'http://ns.adobe.com/xap/1.0/rights/') {
    return [
      { name: 'Marked', label: 'Marked' },
      { name: 'WebStatement', label: 'WebStatement' },
      { name: 'UsageTerms', label: 'UsageTerms' },
      { name: 'Certificate', label: 'Certificate' },
      { name: 'Owner', label: 'Owner' },
    ]
  }
  
  // XMP Media Management namespace
  if (namespaceUri === 'http://ns.adobe.com/xap/1.0/mm/') {
    return [
      { name: 'DocumentID', label: 'DocumentID' },
      { name: 'InstanceID', label: 'InstanceID' },
      { name: 'OriginalDocumentID', label: 'OriginalDocumentID' },
      { name: 'History', label: 'History' },
      { name: 'DerivedFrom', label: 'DerivedFrom' },
    ]
  }
  
  // Photoshop namespace
  if (namespaceUri === 'http://ns.adobe.com/photoshop/1.0/') {
    return [
      { name: 'AuthorsPosition', label: 'AuthorsPosition' },
      { name: 'CaptionWriter', label: 'CaptionWriter' },
      { name: 'Category', label: 'Category' },
      { name: 'City', label: 'City' },
      { name: 'Country', label: 'Country' },
      { name: 'Credit', label: 'Credit' },
      { name: 'DateCreated', label: 'DateCreated' },
      { name: 'Headline', label: 'Headline' },
      { name: 'Instructions', label: 'Instructions' },
      { name: 'Source', label: 'Source' },
      { name: 'State', label: 'State' },
      { name: 'TransmissionReference', label: 'TransmissionReference' },
      { name: 'Urgency', label: 'Urgency' },
    ]
  }
  
  // TIFF namespace
  if (namespaceUri === 'http://ns.adobe.com/tiff/1.0/') {
    return [
      { name: 'ImageWidth', label: 'ImageWidth' },
      { name: 'ImageLength', label: 'ImageLength' },
      { name: 'BitsPerSample', label: 'BitsPerSample' },
      { name: 'Compression', label: 'Compression' },
      { name: 'PhotometricInterpretation', label: 'PhotometricInterpretation' },
      { name: 'Orientation', label: 'Orientation' },
      { name: 'SamplesPerPixel', label: 'SamplesPerPixel' },
      { name: 'PlanarConfiguration', label: 'PlanarConfiguration' },
      { name: 'ResolutionUnit', label: 'ResolutionUnit' },
      { name: 'XResolution', label: 'XResolution' },
      { name: 'YResolution', label: 'YResolution' },
    ]
  }
  
  // Empty list for unknown namespaces
  return []
})

const changeLocale = (val: string) => {
  locale.value = val
  localStorage.setItem('locale', val)
}

const toggleLocale = () => {
  const newLocale = locale.value === 'zh-CN' ? 'en-US' : 'zh-CN'
  changeLocale(newLocale)
}

const localeIcon = computed(() => {
  // Use Switch icon for language switching
  return Switch
})

const localeTooltip = computed(() => {
  return locale.value === 'zh-CN' ? 'Switch to English' : '切换到中文'
})

const toggleTheme = () => {
  // Cycle through: auto -> light -> dark -> auto
  if (theme.value === 'auto') {
    theme.value = 'light'
  } else if (theme.value === 'light') {
    theme.value = 'dark'
  } else {
    theme.value = 'auto'
  }
  localStorage.setItem('theme', theme.value)
  applyTheme(theme.value)
}

const applyTheme = (themeMode: 'auto' | 'light' | 'dark') => {
  const html = document.documentElement
  html.classList.remove('light', 'dark')
  
  if (themeMode === 'light') {
    html.classList.add('light')
  } else if (themeMode === 'dark') {
    html.classList.add('dark')
  }
  // 'auto' means no class, will follow system preference
}

// Computed properties for theme icon and tooltip
const themeIcon = computed(() => {
  if (theme.value === 'light') {
    return Sunny
  } else if (theme.value === 'dark') {
    return Moon
  } else {
    return Monitor
  }
})

const themeTooltip = computed(() => {
  if (theme.value === 'light') {
    return t('common.themeLight')
  } else if (theme.value === 'dark') {
    return t('common.themeDark')
  } else {
    return t('common.themeAuto')
  }
})

// Computed property for logo path based on theme
const logoPath = computed(() => {
  const currentTheme = theme.value === 'auto' 
    ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
    : theme.value
  
  // Use light logo (dark colors) for light theme, and dark logo (bright colors) for dark theme
  // Use BASE_URL to support GitHub Pages subpath deployment
  const baseUrl = import.meta.env.BASE_URL
  return currentTheme === 'light' 
    ? `${baseUrl}assets/logo-icon-light.svg` 
    : `${baseUrl}assets/logo-icon.svg`
})

// Computed property for favicon path based on theme
const faviconPath = computed(() => {
  const currentTheme = theme.value === 'auto' 
    ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
    : theme.value
  
  // Use favicon with circular background optimized for browser tabs
  // Use BASE_URL to support GitHub Pages subpath deployment
  const baseUrl = import.meta.env.BASE_URL
  return currentTheme === 'light' 
    ? `${baseUrl}assets/favicon-light.svg` 
    : `${baseUrl}assets/favicon.svg`
})

// Function to update favicon based on theme
const updateFavicon = (logoPath: string) => {
  let link = document.querySelector("link[rel~='icon']") as HTMLLinkElement
  if (!link) {
    link = document.createElement('link')
    link.rel = 'icon'
    const head = document.getElementsByTagName('head')[0]
    if (head) {
      head.appendChild(link)
    }
  }
  link.href = logoPath
}

// Watch theme changes and update favicon
watch([theme, faviconPath], () => {
  updateFavicon(faviconPath.value)
}, { immediate: true })

const handleFileChange = async (file: { raw: File }) => {
  const result = await loadFile(file.raw)
  if (result) {
    filePreview.value = {
      name: result.fileName,
      size: (file.raw.size / 1024).toFixed(2),
      type: file.raw.type || t('common.fileType'),
      url: URL.createObjectURL(file.raw)
    }
    // Auto-load property value after file is loaded
    if (propertyForm.namespace && propertyForm.property) {
      const value = getPropertyValue(propertyForm.namespace, propertyForm.property)
      if (value !== null) {
        propertyForm.value = value
      } else {
        propertyForm.value = ''
      }
    } else if (propertyForm.namespace && commonPropertyNames.value.length > 0) {
      // If namespace is set but property is not, select first property
      const firstProperty = commonPropertyNames.value[0]
      if (firstProperty) {
        propertyForm.property = firstProperty.name
        const value = getPropertyValue(propertyForm.namespace, firstProperty.name)
        if (value !== null) {
          propertyForm.value = value
        }
      }
    }
  }
}

const handleSetProperty = async () => {
  if (!propertyForm.namespace || !propertyForm.property) {
    return
  }
  const currentNamespace = propertyForm.namespace
  const currentProperty = propertyForm.property
  
  setProperty(propertyForm.namespace, propertyForm.property, propertyForm.value)
  
  // After setting property, reload the value (don't clear the form)
  // Use nextTick to ensure readXmp has completed and reactive updates are done
  await nextTick()
  // Small delay to ensure readXmp completes
  setTimeout(() => {
    if (propertyForm.namespace === currentNamespace && propertyForm.property === currentProperty) {
      const value = getPropertyValue(currentNamespace, currentProperty)
      if (value !== null) {
        propertyForm.value = value
      }
    }
  }, 50)
}

const handleDeleteProperty = () => {
  deleteProperty(propertyForm.namespace, propertyForm.property)
}

const handleRegisterNamespace = () => {
  registerNamespace(namespaceForm.uri, namespaceForm.prefix)
  namespaceForm.uri = ''
  namespaceForm.prefix = ''
}

// Watch namespace changes and auto-select first property with its value
watch(() => propertyForm.namespace, (newNamespace) => {
  if (newNamespace && commonPropertyNames.value.length > 0) {
    // Select the first common property
    const firstProperty = commonPropertyNames.value[0]
    if (firstProperty) {
      propertyForm.property = firstProperty.name
      
      // Try to load the property value if file is loaded
      if (xmpFile.value) {
        const value = getPropertyValue(newNamespace, firstProperty.name)
        if (value !== null) {
          propertyForm.value = value
        } else {
          propertyForm.value = ''
        }
      }
    }
  } else {
    propertyForm.property = ''
    propertyForm.value = ''
  }
})

// Watch property changes and auto-load its value
watch(() => propertyForm.property, (newProperty) => {
  if (newProperty && propertyForm.namespace && xmpFile.value) {
    const value = getPropertyValue(propertyForm.namespace, newProperty)
    if (value !== null) {
      propertyForm.value = value
    } else {
      propertyForm.value = ''
    }
  }
})

onMounted(async () => {
  // Initialize mobile detection
  updateIsMobile()
  window.addEventListener('resize', updateIsMobile)
  
  // Get theme from localStorage or use 'auto'
  const savedTheme = localStorage.getItem('theme') as 'auto' | 'light' | 'dark' | null
  if (savedTheme && ['auto', 'light', 'dark'].includes(savedTheme)) {
    theme.value = savedTheme
  } else {
    theme.value = 'auto'
  }
  applyTheme(theme.value)
  
  // Get locale from localStorage or use system default
  const savedLocale = localStorage.getItem('locale')
  if (savedLocale) {
    locale.value = savedLocale
  } else {
    // Use system language if no saved preference
    const systemLang = navigator.language || navigator.languages?.[0] || 'en-US'
    const detectedLocale = systemLang.startsWith('zh') ? 'zh-CN' : 'en-US'
    locale.value = detectedLocale
    localStorage.setItem('locale', detectedLocale)
  }
  // Initialize WASM and update namespace list
  await initWasm()
  updateRegisteredNamespaces()
  
  // Auto-select custom namespace first, then built-in
  if (!propertyForm.namespace) {
    if (customNamespaces.value.length > 0 && customNamespaces.value[0]) {
      propertyForm.namespace = customNamespaces.value[0].uri
    } else if (builtinNamespaces.value.length > 0 && builtinNamespaces.value[0]) {
      propertyForm.namespace = builtinNamespaces.value[0].uri
    }
  }
  
  // Auto-select first property if namespace is set
  if (propertyForm.namespace && commonPropertyNames.value.length > 0) {
    const firstProperty = commonPropertyNames.value[0]
    if (firstProperty) {
      propertyForm.property = firstProperty.name
      if (xmpFile.value) {
        const value = getPropertyValue(propertyForm.namespace, firstProperty.name)
        if (value !== null) {
          propertyForm.value = value
        }
      }
    }
  }
})
</script>

<style scoped>
.app-container {
  max-width: 1400px;
  margin: 0 auto;
  padding: 24px;
  background: var(--color-background);
  min-height: 100vh;
  transition: background-color 0.3s ease;
  box-sizing: border-box;
  overflow-x: hidden; /* Prevent horizontal scroll */
  width: 100%;
}

.el-header {
  background: var(--color-card-bg);
  border-radius: 12px;
  padding: 24px;
  margin-bottom: 24px;
  box-shadow: 0 2px 8px var(--color-card-shadow);
  height: auto !important;
  display: flex;
  align-items: center;
  min-height: 64px;
  border: 1px solid var(--color-border);
  transition: all 0.3s ease;
  box-sizing: border-box;
  overflow: hidden; /* Prevent content overflow */
  width: 100%;
}

.header-content {
  display: grid;
  grid-template-columns: 1fr auto;
  align-items: center;
  width: 100%;
  max-width: 100%;
  gap: 16px;
  box-sizing: border-box;
  overflow: hidden; /* Prevent overflow */
}

.title-with-logo {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0; /* Allow text truncation */
  overflow: hidden; /* Prevent overflow */
  box-sizing: border-box;
}

.header-logo {
  width: 48px;
  height: 48px;
  flex-shrink: 0;
}

.el-header h1 {
  margin: 0;
  color: var(--color-heading);
  line-height: 1.2;
  font-size: 24px;
  font-weight: 600;
  transition: color 0.3s ease;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100%;
  flex-shrink: 1;
}

.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-shrink: 0; /* Prevent shrinking */
  flex-grow: 0; /* Prevent growing */
  width: fit-content; /* Fixed width based on content */
}

.theme-toggle-btn {
  font-size: 18px;
  transition: transform 0.2s ease;
  flex-shrink: 0;
}

.theme-toggle-btn:hover {
  transform: scale(1.1);
}

.locale-toggle-btn {
  flex-shrink: 0;
  width: 40px;
  height: 40px;
  font-size: 14px;
  font-weight: 600;
}

.theme-toggle-btn {
  flex-shrink: 0;
  width: 40px;
  height: 40px;
}

.el-main {
  padding: 0;
}

.upload-card,
.preview-card,
.controls-card,
.xmp-card,
.namespace-card,
.edit-card,
.packet-card {
  margin-bottom: 24px;
  border-radius: 12px;
  border: 1px solid var(--color-border);
  box-shadow: 0 2px 8px var(--color-card-shadow);
  transition: all 0.3s ease;
  overflow: hidden;
}

.upload-card :deep(.el-card__body),
.preview-card :deep(.el-card__body),
.controls-card :deep(.el-card__body),
.xmp-card :deep(.el-card__body),
.edit-card :deep(.el-card__body),
.packet-card :deep(.el-card__body) {
  background: var(--color-card-bg);
  transition: background-color 0.3s ease;
}

/* Upload tip text - improve visibility in dark mode */
.upload-card :deep(.el-upload__tip) {
  color: var(--color-text);
  transition: color 0.3s ease, opacity 0.3s ease;
}

:root.dark .upload-card :deep(.el-upload__tip) {
  opacity: 0.85;
}

@media (prefers-color-scheme: dark) {
  :root:not(.light) .upload-card :deep(.el-upload__tip) {
    opacity: 0.85;
  }
}

.upload-card :deep(.el-card__header),
.preview-card :deep(.el-card__header),
.controls-card :deep(.el-card__header),
.xmp-card :deep(.el-card__header),
.edit-card :deep(.el-card__header),
.packet-card :deep(.el-card__header) {
  background: var(--color-background-soft);
  border-bottom: 1px solid var(--color-border);
  padding: 16px 20px;
  font-weight: 600;
  color: var(--color-heading);
  transition: all 0.3s ease;
}

.namespace-card {
  background: var(--color-card-bg);
  border: 1px solid var(--color-border);
  border-radius: 12px;
  box-shadow: 0 2px 8px var(--color-card-shadow);
  transition: all 0.3s ease;
}

.namespace-card :deep(.el-collapse-item__header) {
  padding-left: 20px;
  padding-right: 20px;
  background: var(--color-background-soft);
  border-bottom: 1px solid var(--color-border);
  font-weight: 600;
  color: var(--color-heading);
  transition: all 0.3s ease;
}

.namespace-card :deep(.el-collapse-item__content) {
  padding: 20px;
  background: var(--color-card-bg);
  transition: background-color 0.3s ease;
}

.preview-container {
  text-align: center;
  padding: 24px;
  background: var(--color-background-mute);
  border-radius: 8px;
  transition: background-color 0.3s ease;
}

.preview-container img,
.preview-container video {
  max-width: 100%;
  max-height: 600px;
  border-radius: 8px;
  box-shadow: 0 4px 12px var(--color-card-shadow);
  transition: box-shadow 0.3s ease;
}

.preview-container audio {
  width: 100%;
  max-width: 500px;
}

.file-placeholder {
  padding: 40px;
  color: var(--color-text);
  transition: color 0.3s ease;
}

.file-placeholder-tip {
  font-size: 12px;
  margin-top: 10px;
}

.namespace-table-header {
  font-weight: 600;
  margin-bottom: 12px;
  color: var(--color-heading);
  font-size: 14px;
  transition: color 0.3s ease;
}

.namespace-uri {
  font-family: 'Courier New', monospace;
  font-size: 12px;
  color: var(--color-text);
  word-break: break-all;
  opacity: 0.8;
  transition: color 0.3s ease, opacity 0.3s ease;
}

/* Improve visibility in dark mode */
:root.dark .namespace-uri {
  opacity: 0.95;
}

@media (prefers-color-scheme: dark) {
  :root:not(.light) .namespace-uri {
    opacity: 0.95;
  }
}

/* Namespace form inputs */
.namespace-uri-input {
  width: 400px;
  max-width: 100%;
}

.namespace-prefix-input {
  width: 200px;
  max-width: 100%;
}

.xmp-packet {
  background: var(--color-background-mute);
  padding: 16px;
  border-radius: 8px;
  font-size: 13px;
  line-height: 1.6;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  color: var(--color-text);
  border: 1px solid var(--color-border);
  transition: all 0.3s ease;
  font-family: 'Courier New', monospace;
}

/* Enhanced button styles */
:deep(.el-button) {
  border-radius: 8px;
  transition: all 0.3s ease;
  font-weight: 500;
}

:deep(.el-button:hover) {
  transform: translateY(-1px);
  box-shadow: 0 4px 8px var(--color-card-shadow);
}

:deep(.el-button:active) {
  transform: translateY(0);
}

/* Enhanced input styles */
:deep(.el-input__wrapper) {
  border-radius: 8px;
  transition: all 0.3s ease;
  box-shadow: 0 0 0 1px var(--color-border) inset;
}

:deep(.el-input__wrapper:hover) {
  box-shadow: 0 0 0 1px var(--color-border-hover) inset;
}

:deep(.el-input__wrapper.is-focus) {
  box-shadow: 0 0 0 1px var(--el-color-primary) inset;
}

:deep(.el-select .el-input__wrapper) {
  border-radius: 8px;
}

:deep(.el-textarea__inner) {
  border-radius: 8px;
  transition: all 0.3s ease;
}

/* Enhanced table styles */
:deep(.el-table) {
  border-radius: 8px;
  overflow: hidden;
  border: 1px solid var(--color-border);
}

:deep(.el-table th) {
  background: var(--color-background-soft);
  color: var(--color-heading);
  font-weight: 600;
}

:deep(.el-table td) {
  background: var(--color-card-bg);
  color: var(--color-text);
}

:deep(.el-table--striped .el-table__body tr.el-table__row--striped td) {
  background: var(--color-background-mute);
}

:deep(.el-table tr:hover > td) {
  background: var(--color-background-soft);
}

/* Enhanced card hover effects */
.upload-card:hover,
.preview-card:hover,
.controls-card:hover,
.xmp-card:hover,
.edit-card:hover,
.packet-card:hover {
  box-shadow: 0 4px 16px var(--color-card-shadow);
  transform: translateY(-2px);
}

.namespace-card:hover {
  box-shadow: 0 4px 16px var(--color-card-shadow);
  transform: translateY(-2px);
}

/* Enhanced form styles */
:deep(.el-form-item__label) {
  color: var(--color-heading);
  font-weight: 500;
}

:deep(.el-descriptions__label) {
  color: var(--color-heading);
  font-weight: 500;
}

:deep(.el-descriptions__content) {
  color: var(--color-text);
}

/* Enhanced tag styles */
:deep(.el-tag) {
  border-radius: 6px;
  font-weight: 500;
}

/* Enhanced divider styles */
:deep(.el-divider) {
  border-color: var(--color-border);
}

/* Enhanced scrollbar styles */
:deep(.el-scrollbar__bar) {
  opacity: 0.6;
}

:deep(.el-scrollbar__bar:hover) {
  opacity: 1;
}

/* Mobile Responsive Styles */
@media (max-width: 768px) {
  /* Improve touch targets */
  :deep(.el-button),
  :deep(.el-select),
  :deep(.el-input),
  :deep(.el-textarea) {
    min-height: 44px; /* iOS recommended touch target size */
  }

  /* Header adjustments */
  .el-header {
    padding: 12px 16px;
    min-height: 56px;
  }

  .header-content {
    gap: 12px;
    overflow: hidden; /* Prevent overflow */
    max-width: 100%;
  }

  .title-with-logo {
    gap: 8px;
    min-width: 0; /* Allow flex shrinking */
    overflow: hidden; /* Prevent overflow */
    box-sizing: border-box;
  }

  .header-logo {
    width: 36px;
    height: 36px;
  }

  .el-header h1 {
    font-size: 18px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .header-actions {
    gap: 6px;
    flex-shrink: 0;
    flex-grow: 0;
    display: flex;
    align-items: center;
    flex-wrap: nowrap;
    width: fit-content;
  }

  .locale-toggle-btn {
    width: 36px;
    height: 36px;
    font-size: 13px;
    min-width: 36px;
    min-height: 36px;
  }

  .theme-toggle-btn {
    font-size: 16px;
    flex-shrink: 0;
    width: 36px; /* Fixed width for circle button */
    height: 36px;
    min-width: 36px;
    min-height: 36px;
  }

  /* Main content padding */
  .el-main {
    padding: 0 12px;
  }

  /* Card spacing */
  .upload-card,
  .preview-card,
  .controls-card,
  .xmp-card,
  .namespace-card,
  .edit-card,
  .packet-card {
    margin-bottom: 16px;
    border-radius: 8px;
  }

  .upload-card :deep(.el-card__header),
  .preview-card :deep(.el-card__header),
  .controls-card :deep(.el-card__header),
  .xmp-card :deep(.el-card__header),
  .edit-card :deep(.el-card__header),
  .packet-card :deep(.el-card__header) {
    padding: 12px 16px;
    font-size: 14px;
  }

  .upload-card :deep(.el-card__body),
  .preview-card :deep(.el-card__body),
  .controls-card :deep(.el-card__body),
  .xmp-card :deep(.el-card__body),
  .edit-card :deep(.el-card__body),
  .packet-card :deep(.el-card__body) {
    padding: 16px;
  }

  /* Upload area */
  .upload-card :deep(.el-upload-dragger) {
    padding: 30px 20px;
  }

  .upload-card :deep(.el-icon--upload) {
    font-size: 48px;
  }

  .upload-card :deep(.el-upload__text) {
    font-size: 14px;
    margin-top: 12px;
  }

  .upload-card :deep(.el-upload__tip) {
    font-size: 12px;
    margin-top: 8px;
  }

  /* Preview container */
  .preview-container {
    padding: 16px;
  }

  .preview-container img,
  .preview-container video {
    max-height: 400px;
  }

  .preview-container audio {
    max-width: 100%;
  }

  .file-placeholder {
    padding: 24px;
  }

  /* Operation buttons - stack vertically on mobile */
  .controls-card :deep(.el-card__body) {
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: stretch; /* Ensure buttons stretch to full width */
  }

  .controls-card :deep(.el-button) {
    width: 100%;
    justify-content: center;
    padding: 12px 20px;
    font-size: 14px;
    margin: 0; /* Remove any default margins */
  }

  /* Edit buttons - stack vertically on mobile */
  .edit-buttons {
    margin-top: 16px;
    margin-bottom: 0;
  }

  .edit-buttons :deep(.el-form-item__content) {
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: stretch;
  }

  .edit-buttons :deep(.el-button) {
    width: 100%;
    justify-content: center;
    padding: 12px 20px;
    font-size: 14px;
    margin: 0 !important; /* Remove any default margins */
  }

  /* Descriptions - single column on mobile */
  :deep(.el-descriptions) {
    --el-descriptions-table-border: 1px solid var(--color-border);
  }

  :deep(.el-descriptions__label) {
    width: 30% !important;
    font-size: 13px;
    padding: 8px 12px;
  }

  :deep(.el-descriptions__content) {
    font-size: 13px;
    padding: 8px 12px;
    word-break: break-word;
  }

  /* Tables - make scrollable on mobile */
  :deep(.el-table) {
    font-size: 13px;
    overflow-x: auto;
    display: block;
    width: 100%;
  }

  :deep(.el-table__body-wrapper),
  :deep(.el-table__header-wrapper) {
    overflow-x: auto;
  }

  :deep(.el-table th),
  :deep(.el-table td) {
    padding: 8px 12px;
    white-space: nowrap;
  }

  :deep(.el-table th:last-child),
  :deep(.el-table td:last-child) {
    white-space: normal;
    word-break: break-word;
  }

  /* Form inputs */
  :deep(.el-input) {
    font-size: 14px;
  }

  :deep(.el-textarea__inner) {
    font-size: 14px;
    padding: 8px 12px;
  }

  /* Namespace card */
  .namespace-card :deep(.el-collapse-item__header) {
    padding: 12px 16px;
    font-size: 14px;
  }

  .namespace-card :deep(.el-collapse-item__content) {
    padding: 16px;
  }

  /* Namespace form - stack vertically on mobile */
  .namespace-form {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .namespace-form :deep(.el-form-item) {
    margin-bottom: 0;
    width: 100%;
  }

  .namespace-form :deep(.el-form-item__label) {
    width: auto !important;
    margin-bottom: 8px;
    display: block;
  }

  .namespace-uri-input,
  .namespace-prefix-input {
    width: 100% !important;
    max-width: 100%;
  }

  .namespace-table-header {
    font-size: 13px;
    margin-bottom: 8px;
  }

  .namespace-uri {
    font-size: 11px;
  }

  /* XMP Packet display */
  .xmp-packet {
    padding: 12px;
    font-size: 12px;
    line-height: 1.5;
  }

  /* Button groups */
  :deep(.el-button-group) {
    display: flex;
    flex-direction: column;
    width: 100%;
  }

  :deep(.el-button-group .el-button) {
    width: 100%;
    margin-left: 0 !important;
    margin-top: 8px;
  }

  :deep(.el-button-group .el-button:first-child) {
    margin-top: 0;
  }

  /* Dialog adjustments */
  :deep(.el-dialog) {
    width: 90% !important;
    margin: 5vh auto !important;
  }

  :deep(.el-dialog__header) {
    padding: 16px;
  }

  :deep(.el-dialog__body) {
    padding: 16px;
    font-size: 14px;
  }

  :deep(.el-dialog__footer) {
    padding: 12px 16px;
  }

  /* Tag adjustments */
  :deep(.el-tag) {
    font-size: 12px;
    padding: 4px 8px;
  }
}

/* Extra small devices (phones, less than 480px) */
@media (max-width: 480px) {
  .el-header {
    padding: 10px 12px;
    min-height: 52px;
  }

  .header-logo {
    width: 32px;
    height: 32px;
  }

  .el-header h1 {
    font-size: 16px;
  }

  .header-actions {
    gap: 4px;
    flex-shrink: 0;
    flex-grow: 0;
    width: fit-content;
  }

  .locale-toggle-btn {
    width: 32px;
    height: 32px;
    font-size: 12px;
    min-width: 32px;
    min-height: 32px;
  }

  .theme-toggle-btn {
    width: 32px;
    height: 32px;
    font-size: 14px;
    min-width: 32px;
    min-height: 32px;
  }

  .el-main {
    padding: 0 8px;
  }

  .upload-card,
  .preview-card,
  .controls-card,
  .xmp-card,
  .namespace-card,
  .edit-card,
  .packet-card {
    margin-bottom: 12px;
    border-radius: 6px;
  }

  .upload-card :deep(.el-card__header),
  .preview-card :deep(.el-card__header),
  .controls-card :deep(.el-card__header),
  .xmp-card :deep(.el-card__header),
  .edit-card :deep(.el-card__header),
  .packet-card :deep(.el-card__header) {
    padding: 10px 12px;
    font-size: 13px;
  }

  .upload-card :deep(.el-card__body),
  .preview-card :deep(.el-card__body),
  .controls-card :deep(.el-card__body),
  .xmp-card :deep(.el-card__body),
  .edit-card :deep(.el-card__body),
  .packet-card :deep(.el-card__body) {
    padding: 12px;
  }

  .preview-container img,
  .preview-container video {
    max-height: 300px;
  }

  .controls-card :deep(.el-button) {
    padding: 10px 16px;
    font-size: 13px;
  }

  .edit-buttons :deep(.el-button) {
    padding: 10px 16px;
    font-size: 13px;
  }

  :deep(.el-descriptions__label) {
    width: 35% !important;
    font-size: 12px;
    padding: 6px 10px;
  }

  :deep(.el-descriptions__content) {
    font-size: 12px;
    padding: 6px 10px;
  }

  :deep(.el-dialog) {
    width: 95% !important;
  }
}

/* Landscape orientation on mobile */
@media (max-width: 768px) and (orientation: landscape) {
  .preview-container img,
  .preview-container video {
    max-height: 50vh;
  }
}

/* Mobile-specific improvements */
@media (max-width: 768px) {
  /* Smooth scrolling */
  html {
    -webkit-overflow-scrolling: touch;
  }

  /* Prevent text selection on buttons for better touch experience */
  .controls-card :deep(.el-button) {
    -webkit-tap-highlight-color: transparent;
    user-select: none;
  }

  /* Improve upload area touch interaction */
  .upload-card :deep(.el-upload-dragger) {
    -webkit-tap-highlight-color: transparent;
  }

  /* Better scrollbar visibility on mobile */
  :deep(.el-scrollbar__bar) {
    opacity: 0.8;
  }

  /* Prevent horizontal scroll on body */
  body {
    overflow-x: hidden;
  }

  /* Ensure cards don't overflow */
  .upload-card,
  .preview-card,
  .controls-card,
  .xmp-card,
  .namespace-card,
  .edit-card,
  .packet-card {
    max-width: 100%;
    box-sizing: border-box;
  }
}
</style>
