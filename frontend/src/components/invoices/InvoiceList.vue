<template>
  <div class="invoice-list">
    <o-table
      v-if="invoices !== null"
      :data="invoices"
      :backend-pagination="true"
      aria-next-label="Next page"
      aria-previous-label="Previous page"
      aria-page-label="Page"
      aria-current-label="Current page"
    >
      <o-table-column
        field="id"
        label="Rechnungsnr."
        width="20"
        numeric
        v-slot="props"
      >
        {{
          props.row.invoiceNumber !== undefined
            ? props.row.invoiceNumber
            : "Keine"
        }}
      </o-table-column>
      <o-table-column field="title" label="Titel" width="200" v-slot="props">
        {{ props.row.title }}
      </o-table-column>
      <o-table-column
        field="customerContact.name"
        label="Kunde"
        width="200"
        v-slot="props"
      >
        {{ getCustomerName(props.row.customerContact) }}
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
import { type InvoiceListItem } from "@/models/InvoiceModel";
import { listInvoices } from "@/services/InvoicesApiService";
import { ref, type Ref } from "vue";
import { useRoute, useRouter } from "vue-router";

const route = useRoute();
const router = useRouter();
const PAGE_SIZE = 100000;

let pagesCache: Map<number, Array<InvoiceListItem>> = new Map();
const invoices: Ref<Array<InvoiceListItem> | null> = ref(null);

const pageChange = (page: number): void => {
  invoices.value = pagesCache.get(page) ?? null;
  if (invoices.value === null) {
    listInvoices(page, PAGE_SIZE).then((v) => {
      invoices.value = v;
      pagesCache.set(page, v);
    });
  }
}

const view = (id: number): void => {
  router.push(`/invoices/${id}`);
}
const getCustomerName = (customerContact: Contact | undefined): string => {
  return customerContact?.name ?? "";
}
const clearCache = (): void => {
  pagesCache = new Map();
}
const reload = (): void => {
  pageChange(0);
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