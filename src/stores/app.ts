import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

export type Theme = 'light' | 'dark'

const THEME_STORAGE_KEY = 'app-theme'

// 从 localStorage 加载主题
function loadThemeFromStorage(): Theme {
  try {
    const stored = localStorage.getItem(THEME_STORAGE_KEY)
    if (stored === 'light' || stored === 'dark') {
      return stored
    }
  } catch (e) {
    console.error('加载主题失败:', e)
  }
  return 'light'
}

export const useAppStore = defineStore('app', () => {
  // 主题 - 从 localStorage 初始化
  const theme = ref<Theme>(loadThemeFromStorage())

  // 侧边栏折叠状态
  const sidebarCollapsed = ref(false)

  // 监听主题变化并保存到 localStorage
  watch(theme, (newTheme) => {
    try {
      localStorage.setItem(THEME_STORAGE_KEY, newTheme)
    } catch (e) {
      console.error('保存主题失败:', e)
    }
  })

  // 切换主题
  function toggleTheme() {
    theme.value = theme.value === 'light' ? 'dark' : 'light'
  }

  // 设置主题
  function setTheme(newTheme: Theme) {
    theme.value = newTheme
  }

  // 切换侧边栏
  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value
  }

  return {
    theme,
    sidebarCollapsed,
    toggleTheme,
    setTheme,
    toggleSidebar,
  }
})

