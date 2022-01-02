<template>
  <div class="receipts-overview">
    <button class="button is-primary" @click="newReceipt()">Neuer Beleg</button>
    <receipt-list ref="receiptList" />
  </div>
</template>

<script lang="ts">
import { Component, Vue } from "vue-property-decorator";
import ReceiptList from "@/components/receipts/ReceiptList.vue";

@Component({
  name: "receipts-overview",
  components: {
    ReceiptList,
  },
})
export default class ReceiptsOverview extends Vue {
  private newReceipt(): void {
    this.$router.push("/receipts/new");
  }
}
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss"></style>
