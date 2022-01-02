<template>
  <div class="receipts">
    <keep-alive include="receipts-overview">
      <router-view />
    </keep-alive>
  </div>
</template>

<script lang="ts">
import { Component, Vue } from "vue-property-decorator";

@Component({})
export default class Receipts extends Vue {}
</script>

<style lang="scss" scoped>
.receipts {
  padding-top: 10px;
}
</style>
