<template>
  <div class="invoice-editor" v-if="invoice !== null">
    <div>
      <div class="top-buttons">
        <o-button type="is-info" @click="back">Zurück</o-button>
        <o-button type="is-success" @click="save">Speichern</o-button>
        <o-button v-if="!isCommitted" type="is-danger" @click="commit"
          >Festschreiben</o-button
        >
        <o-button
          v-if="isCommitted && invoice.document === undefined"
          type="is-warning"
          :loading="isExporting"
          :disabled="invoice.id === undefined"
          @click="tryExport"
          >Exportieren</o-button
        >
        <o-button
          v-if="isCommitted && invoice.document !== undefined"
          type="is-warning"
          tag="a"
          :href="`/api/documents/${invoice.document.id}`"
          target="_blank"
          :download="`Rechnung ${invoice.invoiceNumber}.pdf`"
          >PDF Herunterladen</o-button
        >
      </div>
    </div>
    <o-field label="Titel">
      <o-input @input="change" v-model="invoice.title" />
    </o-field>
    <o-field label="Kunde">
      <contact-search
        :contact="
          invoice.customerContact === undefined ? null : invoice.customerContact
        "
        :disabled="isCommitted"
        @select="select"
      />
    </o-field>
    <recipient-editor
      @change="change"
      :disabled="isCommitted"
      v-model="invoice.recipient"
    />
    <o-field grouped>
      <o-field expanded label="Rechnungsdatum">
        <o-datepicker
          @input="change"
          :disabled="isCommitted"
          v-model="invoice.invoiceDate"
        />
        <p class="control">
          <o-button
            @click="
              invoice.invoiceDate = undefined;
              change();
            "
            icon-right="delete"
            :disabled="invoice.invoiceDate === undefined || isCommitted"
          />
        </p>
      </o-field>
      <o-field expanded label="Bezahlt am">
        <o-datepicker @input="change" v-model="invoice.paidDate" />
        <p class="control">
          <o-button
            @click="
              invoice.paidDate = undefined;
              change();
            "
            icon-right="delete"
            :disabled="invoice.paidDate === undefined"
          />
        </p>
      </o-field>
    </o-field>
    <o-field label="Betreff">
      <o-input
        @input="change"
        :disabled="isCommitted"
        v-model="invoice.subject"
      />
    </o-field>
    <o-field label="Einleitungstext">
      <o-input
        @input="change"
        :disabled="isCommitted"
        type="textarea"
        v-model="invoice.headerHTML"
      />
    </o-field>
    <items-editor
      @change="change"
      :disabled="isCommitted"
      v-model="invoice.items"
    />
    <p>Gesamt: {{ getTotal() }}</p>
    <o-field label="Fußtext">
      <o-input
        @input="change"
        :disabled="isCommitted"
        type="textarea"
        v-model="invoice.footerHTML"
      />
    </o-field>
  </div>
</template>

<script setup lang="ts">

import { ref, computed } from "vue";
import {
  commitInvoice,
  createInvoice,
  exportInvoice,
  fetchInvoice,
  updateInvoice,
} from "@/services/InvoicesApiService";
import { type Contact } from "@/models/ContactModel";
import { formatCentsAsMoney } from "@/util/MoneyUtil";
import { type Invoice } from "@/models/InvoiceModel";
import { parseISO } from "date-fns";
import ContactSearch from "../common/ContactSearch.vue";
import ItemsEditor from "../common/ItemsEditor.vue";
import RecipientEditor from "../common/RecipientEditor.vue";
import { useRoute, useRouter } from "vue-router";
import { useProgrammatic } from "@oruga-ui/oruga-next";


const { oruga } = useProgrammatic();

  const route = useRoute();
  const router = useRouter();
  const invoice = ref<Invoice | null>(null);
  const changed = ref(false); //TODO: Warn if exporting with unsaved changes
  const changedSinceSave = ref(false);
  const isExporting = ref(false);
  const change = ()  => {
    changedSinceSave.value = true;
  }
  const isCommitted = computed((): boolean => {
    return invoice.value?.committedTimestamp !== undefined;
  });
  const commit = (): void => {
    if (invoice.value !== null && invoice.value?.id !== undefined) {
      commitInvoice(invoice.value.id).then((response) => {
        if (invoice.value !== null) {
          invoice.value.committedTimestamp = parseISO(
            response.committedTimestamp
          );
          invoice.value.invoiceNumber = response.invoiceNumber;
        }
      });
    }
  }
  const getTotal = (): string => {
    let total = 0;
    if (invoice.value !== null) {
      invoice.value.items.forEach((item) => {
        total += Number.parseInt(
          (item.price.amountCents * item.quantity).toFixed(0)
        );
      });
    }
    return formatCentsAsMoney(total);
  }
  const exportDocument = ()  => {
    if (invoice.value !== null) {
      const startInvoiceId = invoice.value.id;
      exportInvoice(invoice.value)
        .then((r) => {
          if (invoice.value !== null && invoice.value?.id === startInvoiceId) {
            invoice.value.document = r.document;
          }
          isExporting.value = false;
          oruga.toast.open({
            message: "Export erfolgreich",
            type: "is-success",
          });
        })
        .catch(() => {
          isExporting.value = false;
          oruga.toast.open({
            message: "Fehler beim Export",
            type: "is-danger",
          });
        });
      isExporting.value = true;
    }
  }
  const tryExport = ()  => {
    if (changedSinceSave.value) {
      oruga.dialog.confirm({
        message:
          "Die Rechnung enthält ungespeicherte Änderungen!\n" +
          "Trotzdem exportieren (ohne ungespeicherte Änderungen)?",
        title: "Ungespeicherte Änderungen",
        onConfirm: exportDocument,
        trapFocus: true,
        canCancel: true,
        confirmText: "Ja",
        cancelText: "Abbrechen",
      });
    } else {
      exportDocument();
    }
  };
  
  const back = ()  => {
    router.push({
      path: "/invoices",
      query: { forceRefresh: changed.value.toString() },
    });
  }
  const select = (option: Contact)  => {
    if (invoice.value !== null) {
      invoice.value.customerContact = option;
      invoice.value.recipient!.formOfAddress = option.formOfAddress;
      invoice.value.recipient!.title = option.title;
      invoice.value.recipient!.name = option.name;
      invoice.value.recipient!.firstName = option.firstName;
      invoice.value.recipient!.street = option.street;
      invoice.value.recipient!.zipCode = option.zipCode;
      invoice.value.recipient!.city = option.city;
      invoice.value.recipient!.houseNumber = option.houseNumber;
      invoice.value.recipient!.country = option.country;
    }
    change();
  }
  const save = (): void => {
    if (invoice.value !== null && invoice.value?.id === undefined) {
      createInvoice(invoice.value).then((result) => {
        history.replaceState(
          history.state,
          document.title,
          "/invoices/" + result.id
        );
        invoice.value = result;
        changedSinceSave.value = false;
      });
    } else if (invoice.value !== null) {
      updateInvoice(invoice.value).then(() => (changedSinceSave.value = false));
    }
    changed.value = true;
  }
  const created = (): void => {
    const id = route.params["id"] as string;
    if (id === "new") {
      invoice.value = {
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
        invoice.value = v;
      });
    }
  }
  void created();
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