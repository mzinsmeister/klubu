<template>
  <div class="invoice-editor" v-if="invoice !== null">
    <div>
      <div class="top-buttons">
        <b-button type="is-info" @click="back">Zurück</b-button>
        <b-button type="is-success" @click="save">Speichern</b-button>
        <b-button v-if="!isCommitted" type="is-danger" @click="commit"
          >Festschreiben</b-button
        >
        <b-button
          v-if="isCommitted && invoice.document === undefined"
          type="is-warning"
          :loading="isExporting"
          :disabled="invoice.id === undefined"
          @click="tryExport"
          >Exportieren</b-button
        >
        <b-button
          v-if="isCommitted && invoice.document !== undefined"
          type="is-warning"
          tag="a"
          :href="`http://localhost:8081/api/documents/${invoice.document.id}`"
          target="_blank"
          :download="`Rechnung ${invoice.invoiceNumber}.pdf`"
          >PDF Herunterladen</b-button
        >
      </div>
    </div>
    <b-field label="Titel">
      <b-input @input="change" v-model="invoice.title" />
    </b-field>
    <b-field label="Kunde">
      <contact-search
        :contact="
          invoice.customerContact === undefined ? null : invoice.customerContact
        "
        :disabled="isCommitted"
        @select="select"
      />
    </b-field>
    <recipient-editor
      @change="change"
      :disabled="isCommitted"
      v-model="invoice.recipient"
    />
    <b-field grouped>
      <b-field expanded label="Rechnungsdatum">
        <b-datepicker
          @input="change"
          :disabled="isCommitted"
          v-model="invoice.invoiceDate"
        />
        <p class="control">
          <b-button
            @click="
              invoice.invoiceDate = undefined;
              change();
            "
            icon-right="delete"
            :disabled="invoice.invoiceDate === undefined || isCommitted"
          />
        </p>
      </b-field>
      <b-field expanded label="Bezahlt am">
        <b-datepicker @input="change" v-model="invoice.paidDate" />
        <p class="control">
          <b-button
            @click="
              invoice.paidDate = undefined;
              change();
            "
            icon-right="delete"
            :disabled="invoice.paidDate === undefined"
          />
        </p>
      </b-field>
    </b-field>
    <b-field label="Betreff">
      <b-input
        @input="change"
        :disabled="isCommitted"
        v-model="invoice.subject"
      />
    </b-field>
    <b-field label="Einleitungstext">
      <b-input
        @input="change"
        :disabled="isCommitted"
        type="textarea"
        v-model="invoice.headerHTML"
      />
    </b-field>
    <items-editor
      @change="change"
      :disabled="isCommitted"
      v-model="invoice.items"
    />
    <p>Gesamt: {{ getTotal() }}</p>
    <b-field label="Fußtext">
      <b-input
        @input="change"
        :disabled="isCommitted"
        type="textarea"
        v-model="invoice.footerHTML"
      />
    </b-field>
  </div>
</template>

<script lang="ts">
import { Contact } from "@/models/ContactModel";
import { Invoice } from "@/models/InvoiceModel";
import RecipientEditor from "../common/RecipientEditor.vue";
import ContactSearch from "../common/ContactSearch.vue";
import ItemsEditor from "../common/ItemsEditor.vue";
import {
  commitInvoice,
  createInvoice,
  exportInvoice,
  fetchInvoice,
  updateInvoice,
} from "@/services/InvoicesApiService";
import { formatCentsAsMoney } from "@/util/MoneyUtil";
import { Component, Vue } from "vue-property-decorator";
import { parseISO } from "date-fns";

@Component({
  name: "invoice-editor",
  components: {
    RecipientEditor,
    ContactSearch,
    ItemsEditor,
  },
})
export default class InvoiceEditor extends Vue {
  private invoice: Invoice | null = null;
  private changed = false; //TODO: Warn if exporting with unsaved changes
  private changedSinceSave = false;

  private isExporting = false;

  private change() {
    this.changedSinceSave = true;
  }

  private get isCommitted(): boolean {
    return this.invoice?.committedTimestamp !== undefined;
  }

  private commit(): void {
    if (this.invoice !== null && this.invoice?.id !== undefined) {
      commitInvoice(this.invoice.id).then((response) => {
        if (this.invoice !== null) {
          this.invoice.committedTimestamp = parseISO(
            response.committedTimestamp
          );
          this.invoice.invoiceNumber = response.invoiceNumber;
        }
      });
    }
  }

  private getTotal(): string {
    let total = 0;
    if (this.invoice !== null) {
      this.invoice.items.forEach((item) => {
        total += Number.parseInt(
          (item.price.amountCents * item.quantity).toFixed(0)
        );
      });
    }
    return this.formatCentsAsMoney(total);
  }

  private formatCentsAsMoney(cents: number): string {
    return formatCentsAsMoney(cents);
  }

  private tryExport() {
    if (this.changedSinceSave) {
      this.$buefy.dialog.confirm({
        message:
          "Die Rechnung enthält ungespeicherte Änderungen!\n" +
          "Trotzdem exportieren (ohne ungespeicherte Änderungen)?",
        title: "Ungespeicherte Änderungen",
        onConfirm: this.export,
        trapFocus: true,
        canCancel: true,
        confirmText: "Ja",
        cancelText: "Abbrechen",
      });
    } else {
      this.export();
    }
  }

  private export() {
    if (this.invoice !== null) {
      const startInvoiceId = this.invoice.id;
      exportInvoice(this.invoice)
        .then((r) => {
          if (this.invoice !== null && this.invoice?.id === startInvoiceId) {
            this.invoice.document = r.document;
          }
          this.isExporting = false;
          this.$buefy.toast.open({
            message: "Export erfolgreich",
            type: "is-success",
          });
        })
        .catch(() => {
          this.isExporting = false;
          this.$buefy.toast.open({
            message: "Fehler beim Export",
            type: "is-danger",
          });
        });
      this.isExporting = true;
    }
  }

  private back() {
    this.$router.push({
      path: "/invoices",
      query: { forceRefresh: this.changed.toString() },
    });
  }

  private select(option: Contact) {
    if (this.invoice !== null) {
      this.invoice.customerContact = option;
      this.invoice.recipient!.formOfAddress = option.formOfAddress;
      this.invoice.recipient!.title = option.title;
      this.invoice.recipient!.name = option.name;
      this.invoice.recipient!.firstName = option.firstName;
      this.invoice.recipient!.street = option.street;
      this.invoice.recipient!.zipCode = option.zipCode;
      this.invoice.recipient!.city = option.city;
      this.invoice.recipient!.houseNumber = option.houseNumber;
      this.invoice.recipient!.country = option.country;
    }
    this.change();
  }

  private save(): void {
    if (this.invoice !== null && this.invoice?.id === undefined) {
      createInvoice(this.invoice).then((result) => {
        history.replaceState(
          history.state,
          document.title,
          "/invoices/" + result.id
        );
        this.invoice = result;
        this.changedSinceSave = false;
      });
    } else if (this.invoice !== null) {
      updateInvoice(this.invoice).then(() => (this.changedSinceSave = false));
    }
    this.changed = true;
  }

  private created(): void {
    const id = this.$route.params["id"];
    if (id === "new") {
      this.invoice = {
        items: [],
        subject: "Rechnung",
        isCanceled: false,
        isCancelation: false,
        recipient: { name: "" },
      };
    } else {
      fetchInvoice(Number.parseInt(id)).then((v) => {
        if (v.recipient === undefined) {
          v.recipient = { name: "" };
        }
        this.invoice = v;
      });
    }
  }
}
</script>

<style scoped lang="scss">
.position-input {
  margin-left: auto;
  margin-right: auto;
}
.top-buttons {
  display: flex;
  justify-content: space-between;
  padding: 10px;
}
</style>
