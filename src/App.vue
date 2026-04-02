<template>
  <a-config-provider :theme="themeConfig">
    <div id="app" :class="{ 'dark-mode': isDark }" @contextmenu.prevent>
      <router-view />
    </div>
  </a-config-provider>
</template>

<script setup lang="ts">
import { computed, watch } from 'vue'
import { theme as antTheme } from 'ant-design-vue'
import { useAppStore } from '@/stores/app'

const appStore = useAppStore()
const isDark = computed(() => appStore.theme === 'dark')

const themeConfig = computed(() => ({
  algorithm: isDark.value ? antTheme.darkAlgorithm : antTheme.defaultAlgorithm,
}))

// 同步 dark-mode 类到 body，以便 Modal 等 teleport 组件能正确应用暗色主题
watch(isDark, (dark) => {
  if (dark) {
    document.body.classList.add('dark-mode')
  } else {
    document.body.classList.remove('dark-mode')
  }
}, { immediate: true })
</script>

<style>
#app {
  width: 100%;
  height: 100vh;
  overflow: hidden;
}

.dark-mode {
  background-color: #141414;
  color: rgba(255, 255, 255, 0.85);
}
</style>

