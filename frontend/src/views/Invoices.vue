<template>
  <div class="invoices">
    <keep-alive include="invoices-overview">
      <router-view />
    </keep-alive>
  </div>
</template>

<script lang="ts">
import { Component, Vue } from "vue-property-decorator";

@Component({})
export default class Invoices extends Vue {}
</script>

<style lang="scss" scoped>
.invoices {
  padding-top: 10px;
}
</style>
