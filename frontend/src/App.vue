<template>
  <div id="app">
    <sidebar />
    <router-view class="content" />
  </div>
</template>
<script setup lang="ts">
import Sidebar from "@/components/KlubuSidebar.vue";

</script>
<style lang="scss" scoped>
#app {
  font-family: Avenir, Helvetica, Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-align: center;
  color: $text;
}
.content {
  padding-left: $sidebar-width;
}
</style>