<template>
  <div class="invoice-list">
    <b-table
      v-if="invoices !== null"
      :data="invoices"
      :backend-pagination="true"
      aria-next-label="Next page"
      aria-previous-label="Previous page"
      aria-page-label="Page"
      aria-current-label="Current page"
    >
      <b-table-column
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
      </b-table-column>
      <b-table-column field="title" label="Titel" width="200" v-slot="props">
        {{ props.row.title }}
      </b-table-column>
      <b-table-column
        field="customerContact.name"
        label="Kunde"
        width="200"
        v-slot="props"
      >
        {{ getCustomerName(props.row.customerContact) }}
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
import { InvoiceListItem } from "@/models/InvoiceModel";
import { listInvoices } from "@/services/InvoicesApiService";
import { Component, Vue } from "vue-property-decorator";

const PAGE_SIZE = 100000;

@Component({
  name: "invoice-list",
})
export default class InvoiceList extends Vue {
  private pagesCache: Map<number, Array<InvoiceListItem>> = new Map();
  private invoices: Array<InvoiceListItem> | null = null;

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
    this.$router.push(`/invoices/${id}`);
  }

  private getCustomerName(customerContact: Contact | undefined): string {
    return customerContact?.name ?? "";
  }

  clearCache(): void {
    this.pagesCache = new Map();
  }

  reload(): void {
    this.pageChange(0);
  }

  private pageChange(page: number): void {
    this.invoices = this.pagesCache.get(page) ?? null;
    if (this.invoices === null) {
      listInvoices(page, PAGE_SIZE).then((v) => {
        this.invoices = v;
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
