<template>
  <div class="invoices-overview">
    <button class="button is-primary" @click="newInvoice()">
      Neue Rechnung
    </button>
    <invoice-list ref="invoiceList" />
  </div>
</template>

<script setup lang="ts">

import InvoiceList from "@/components/invoices/InvoiceList.vue";
import { useRouter } from "vue-router";



const router = useRouter();
const newInvoice = (): void => {
  router.push("/invoices/new");
}
</script>
<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss"></style>