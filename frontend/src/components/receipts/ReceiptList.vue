<template>
  <div class="receipt-list">
    <b-table
      v-if="receipts !== null"
      :data="receipts"
      :backend-pagination="true"
      aria-next-label="Next page"
      aria-previous-label="Previous page"
      aria-page-label="Page"
      aria-current-label="Current page"
    >
      <b-table-column
        field="id"
        label="Belegsnr."
        width="20"
        numeric
        v-slot="props"
      >
        {{ props.row.receiptNumber }}
      </b-table-column>
      <!--<b-table-column field="title" label="Titel" width="200" v-slot="props">
        {{ props.row.title }}
      </b-table-column>-->
      <b-table-column
        field="supplierContact.name"
        label="Lieferant"
        width="200"
        v-slot="props"
      >
        {{ getSupplierName(props.row.supplierContact) }}
      </b-table-column>
      <b-table-column custom-key="actions" v-slot="props">
        <button class="button is-small is-light" @click="view(props.row.id)">
          Öffnen
        </button>
      </b-table-column>
    </b-table>
  </div>
</template>

<script lang="ts">
import { Contact } from "@/models/ContactModel";
import { ReceiptListItem } from "@/models/ReceiptModel";
import { listReceipts } from "@/services/ReceiptsApiService";
import { Component, Vue } from "vue-property-decorator";

const PAGE_SIZE = 100000;

@Component({
  name: "receipt-list",
})
export default class ReceiptList extends Vue {
  private pagesCache: Map<number, Array<ReceiptListItem>> = new Map();
  private receipts: Array<ReceiptListItem> | null = null;

  private created(): void {
    this.pageChange(0);
  }

  private activated(): void {
    if (this.$route.query["forceRefresh"] === "true") {
      this.clearCache();
      this.reload();
    }
  }

  private view(id: number): void {
    this.$router.push(`/receipts/${id}`);
  }

  private getSupplierName(supplierContact: Contact | undefined): string {
    return supplierContact?.name ?? "";
  }

  clearCache(): void {
    this.pagesCache = new Map();
  }

  reload(): void {
    this.pageChange(0);
  }

  private pageChange(page: number): void {
    this.receipts = this.pagesCache.get(page) ?? null;
    if (this.receipts === null) {
      listReceipts(page, PAGE_SIZE).then((v) => {
        this.receipts = v;
        this.pagesCache.set(page, v);
      });
    }
  }
}
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss">
h3 {
  margin: 40px 0 0;
}
ul {
  list-style-type: none;
  padding: 0;
}
li {
  display: inline-block;
  margin: 0 10px;
}
a {
  color: $text-invert;
}
</style>
