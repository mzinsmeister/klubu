<template>
  <div class="offers-overview">
    <button class="button is-primary" @click="newOffer()">Neues Angebot</button>
    <offer-list ref="offerList" />
  </div>
</template>

<script lang="ts">
import { Component, Vue } from "vue-property-decorator";
import OfferList from "@/components/offers/OfferList.vue";

@Component({
  name: "offers-overview",
  components: {
    OfferList,
  },
})
export default class OffersOverview extends Vue {
  private newOffer(): void {
    this.$router.push("/offers/new");
  }
}
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss"></style>
