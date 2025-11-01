import { defineStore } from 'pinia'
import { ref } from 'vue'

export type Theme = 'light' | 'dark'

export const useAppStore = defineStore('app', () => {
  // 主题
  const theme = ref<Theme>('light')

  // 侧边栏折叠状态
  const sidebarCollapsed = ref(false)

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

