<template>
  <div class="receipts-overview">
    <button class="button is-primary" @click="newReceipt()">Neuer Beleg</button>
    <receipt-list ref="receiptList" />
  </div>
</template>

<script setup lang="ts">

import ReceiptList from "@/components/receipts/ReceiptList.vue";
import { useRouter } from "vue-router";



const router = useRouter();
const newReceipt = (): void => {
  router.push("/receipts/new");
}
</script>
<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss"></style>