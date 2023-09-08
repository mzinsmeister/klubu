<template>
  <div class="invoices">
    <router-view v-slot="{ Component }">
      <keep-alive include="invoices-overview">
        <component :is="Component" />
      </keep-alive>
    </router-view>
  </div>
</template>

<script setup lang="ts">
</script>
<style lang="scss" scoped>
.invoices {
  padding-top: 10px;
}
</style>