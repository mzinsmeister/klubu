<template>
  <div class="offers">
    <keep-alive include="offers-overview">
      <router-view />
    </keep-alive>
  </div>
</template>

<script lang="ts">
import { Component, Vue } from "vue-property-decorator";

@Component({})
export default class Offers extends Vue {

}
</script>

<style lang="scss" scoped>
.offers {
  padding-top: 10px;
}
</style>
