<template>
  <div class="invoices-overview">
    <button class="button is-primary" @click="newInvoice()">
      Neue Rechnung
    </button>
    <invoice-list ref="invoiceList" />
  </div>
</template>

<script lang="ts">
import { Component, Vue } from "vue-property-decorator";
import InvoiceList from "@/components/invoices/InvoiceList.vue";

@Component({
  name: "invoices-overview",
  components: {
    InvoiceList,
  },
})
export default class InvoicesOverview extends Vue {
  private newInvoice(): void {
    this.$router.push("/invoices/new");
  }
}
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss"></style>
