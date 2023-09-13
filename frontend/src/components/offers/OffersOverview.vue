<template>
  <div class="offers-overview">
    <button class="button is-primary" @click="newOffer()">Neues Angebot</button>
    <offer-list ref="offerList" />
  </div>
</template>

<script setup lang="ts">

import OfferList from "@/components/offers/OfferList.vue";
import { useRouter } from "vue-router";



const router = useRouter();
const newOffer = (): void => {
  router.push("/offers/new");
}
</script>
<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss"></style>