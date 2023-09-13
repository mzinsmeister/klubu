<template>
  <div class="receipt-list">
    <o-table
      v-if="receipts !== null"
      :data="receipts"
      :backend-pagination="true"
      aria-next-label="Next page"
      aria-previous-label="Previous page"
      aria-page-label="Page"
      aria-current-label="Current page"
    >
      <o-table-column
        field="id"
        label="Belegsnr."
        width="20"
        numeric
        v-slot="props"
      >
        {{ props.row.receiptNumber }}
      </o-table-column>
      <!--o-table-column field="title" label="Titel" width="200" v-slot="props">
        {{ props.row.title }}
      </o-table-column>-->
      <o-table-column
        field="supplierContact.name"
        label="Lieferant"
        width="200"
        v-slot="props"
      >
        {{ getSupplierName(props.row.supplierContact) }}
      </o-table-column>
      <o-table-column custom-key="actions" v-slot="props">
        <button class="button is-small is-light" @click="view(props.row.id)">
          Öffnen
        </button>
      </o-table-column>
    </o-table>
  </div>
</template>

<script setup lang="ts">

import { type Contact } from "@/models/ContactModel";
import { listReceipts } from "@/services/ReceiptsApiService";
import { type ReceiptListItem } from "@/models/ReceiptModel";
import { useRoute, useRouter } from "vue-router";
import { ref, type Ref } from "vue";

const route = useRoute();
const router = useRouter();
const PAGE_SIZE = 100000;
let pagesCache: Map<number, Array<ReceiptListItem>> = new Map();
const receipts: Ref<Array<ReceiptListItem> | null> = ref(null);
const view = (id: number): void => {
  router.push(`/receipts/${id}`);
}
const getSupplierName = (supplierContact: Contact | undefined): string => {
  return supplierContact?.name ?? "";
}
const clearCache = (): void => {
  pagesCache = new Map();
}
const reload = (): void => {
  pageChange(0);
}
const pageChange = (page: number): void => {
  receipts.value = pagesCache.get(page) ?? null;
  if (receipts.value === null) {
    listReceipts(page, PAGE_SIZE).then((v) => {
      receipts.value = v;
      pagesCache.set(page, v);
    });
  }
}
if (route.query["forceRefresh"] === "true") {
  clearCache();
  reload();
}
pageChange(0);
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