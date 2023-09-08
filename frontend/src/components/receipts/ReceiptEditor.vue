<template>
  <div class="receipt-editor" v-if="receipt !== null">
    <div>
      <div class="top-buttons">
        <o-button type="is-info" @click="back">Zurück</o-button>
        <o-button type="is-success" @click="save">Speichern</o-button>
        <o-button
          :disabled="isCommitted || receipt.id === undefined"
          type="is-danger"
          @click="commit"
          >Festschreiben</o-button
        >
      </div>
    </div>
    <div class="columns is-vcentered">
      <div class="column is-8">
        <div v-if="receipt.documentData !== null || pdfSrc !== null">
          <div class="pdf-viewer">
            <VuePDF :pdf="pdfSrc" />
          </div>
          <div class="pdf-viewer-controls">
            <o-button type="is-danger" @click="removeDocument"
              >Dokument entfernen</o-button
            >
          </div>
        </div>
        <o-upload
          v-if="receipt.documentData === null && pdfSrc == null"
          v-model="fileUpload"
          accept=".pdf"
          drag-drop
          @input="changeFile"
        >
          <section class="section">
            <div class="content has-text-centered">
              <p>
                <o-icon icon="upload" size="is-large"> </o-icon>
              </p>
              <p>Datei hier hin ziehen oder klicken um Datei auszuwählen</p>
            </div>
          </section>
        </o-upload>
      </div>
      <div class="column inputcolumn">
        <o-field label="Belegnummer">
          <o-input v-model="receipt.receiptNumber" :disabled="isCommitted" />
        </o-field>
        <o-field label="Lieferant">
          <contact-search
            :contact="
              receipt.supplierContact === undefined
                ? null
                : receipt.supplierContact
            "
            :disabled="isCommitted"
            @select="select"
          />
        </o-field>
        <o-field expanded label="Belegdatum">
          <o-datepicker
            @input="change"
            :disabled="isCommitted"
            v-model="receipt.receiptDate"
            expanded
          />
          <p class="control">
            <o-button
              @click="
                receipt.receiptDate = undefined;
                change();
              "
              icon-right="delete"
              :disabled="receipt.receiptDate === undefined || isCommitted"
            />
          </p>
        </o-field>
        <o-field label="Bezahlt am">
          <o-datepicker
            @input="change"
            v-model="receipt.paidDate"
            :disabled="isCommitted"
            expanded
          />
          <p class="control">
            <o-button
              @click="
                receipt.paidDate = undefined;
                change();
              "
              icon-right="delete"
              :disabled="receipt.paidDate === undefined || isCommitted"
            />
          </p>
        </o-field>
        <o-field label="Zu bezahlen bis">
          <o-datepicker
            @input="change"
            v-model="receipt.dueDate"
            :disabled="isCommitted"
            expanded
          />
          <p class="control">
            <o-button
              @click="
                receipt.dueDate = undefined;
                change();
              "
              icon-right="delete"
              :disabled="receipt.dueDate === undefined || isCommitted"
            />
          </p>
        </o-field>
        <div style="height:10px; border-bottom: 1px solid black" />
        <receipt-items-editor
          @change="change"
          :disabled="isCommitted"
          v-model="receipt.items"
        />
        <p>Gesamt: {{ getTotal() }}</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">

import { ref, computed } from "vue";
import {
  commitReceipt,
  createReceipt,
  fetchReceipt,
  updateReceipt,
} from "@/services/ReceiptsApiService";
import { type Contact } from "@/models/ContactModel";
import { formatCentsAsMoney } from "@/util/MoneyUtil";
import { parseISO } from "date-fns";
import { type Receipt } from "@/models/ReceiptModel";
import { VuePDF, usePDF } from "@tato30/vue-pdf";
import ContactSearch from "../common/ContactSearch.vue";
import pdf from "pdfjs-dist";
import ReceiptItemsEditor from "./ReceiptItemsEditor.vue";
import { useRoute, useRouter } from "vue-router";
import { useProgrammatic } from "@oruga-ui/oruga-next";

const { oruga } = useProgrammatic();

const route = useRoute();
const router = useRouter();
const receipt = ref<Receipt | null>(null);
const changed = ref(false);
const changedSinceSave = ref(false);
const fileUpload = ref<File | null>(null);
const pdfNumPages = ref(0);
const pdfSrc = ref<any | null>(null);
const documentChanged = ref(false);
const change = ()  => {
  changedSinceSave.value = true;
}
const isCommitted = computed((): boolean => {
  return receipt.value?.committedTimestamp !== undefined;
});
const changeFile = (file: File)  => {
  file.arrayBuffer().then((b) => {
    if (receipt.value !== null) {
      receipt.value.documentData = {
        data: new Uint8Array(b),
        mediaType: "application/pdf",
      };
      pdfSrc.value = usePDF(receipt.value.documentData.data);
      pdfSrc.value!.promise.then((r: { numPages: number; }) => {
        pdfNumPages.value = r.numPages;
      });
    }
    documentChanged.value = true;
  });
  change();
}
const removeDocument = ()  => {
  if (receipt.value !== null) {
    receipt.value.documentData = null;
    pdfSrc.value = null;
    documentChanged.value = true;
    change();
  }
}
const commit = (): void => {
  if (receipt.value !== null && receipt.value?.id !== undefined) {
    commitReceipt(receipt.value.id).then((response: { committedTimestamp: string; }) => {
      if (receipt.value !== null) {
        receipt.value.committedTimestamp = parseISO(
          response.committedTimestamp
        );
      }
    });
  }
}
const getTotal = (): string => {
  let total = 0;
  if (receipt.value !== null) {
    receipt.value.items.forEach((item) => {
      total += Number.parseInt(item.price.amountCents.toString());
    });
  }
  return formatCentsAsMoney(total);
}
const back = ()  => {
  router.push({
    path: "/receipts",
    query: { forceRefresh: changed.value.toString() },
  });
}
const select = (option: Contact)  => {
  if (receipt.value !== null) {
    receipt.value.supplierContact = option;
  }
  change();
}
const save = (): void => {
  if (receipt.value?.items.some(it => it.category === undefined)) {
    oruga.toast.open({
          message: "Bitte für alle Positionen Kategorien auswählen",
          type: "is-danger",
        });
    return;
  }
  if (receipt.value !== null && receipt.value?.id === undefined) {
    createReceipt(receipt.value).then((result: Receipt | null) => {
      if (result !== null) {
        history.replaceState(
          history.state,
          document.title,
          "/receipts/" + result.id
        );
      }
      receipt.value = result;
      changedSinceSave.value = false;
      documentChanged.value = false;
    });
  } else if (receipt.value !== null) {
    updateReceipt(receipt.value, documentChanged.value).then(() => {
      changedSinceSave.value = false;
      documentChanged.value = false;
    });
  }
  changed.value = true;
}
const id = route.params["id"] as string;
if (id === "new") {
  receipt.value = {
    items: [],
    receiptNumber: "",
    documentData: null,
  };
} else {
  fetchReceipt(Number.parseInt(id)).then((v: Receipt | null) => {
    receipt.value = v;
    if (v !== null && v.document !== undefined) {
      pdfSrc.value = usePDF(`/api/documents/${v.document.id}`);
      pdfSrc.value!.promise.then((r: { numPages: number; }) => {
        pdfNumPages.value = r.numPages;
      });
    }
  });
}
</script>
<style scoped lang="scss">
.receipt-editor {
  margin-left: 2%;
  margin-right: 2%;
}
.inputcolumn {
  height: 75vh;
  overflow-y: scroll;
}
.position-input {
  margin-left: auto;
  margin-right: auto;
}
.top-buttons {
  display: flex;
  justify-content: space-between;
  padding: 10px;
}
.pdf-viewer {
  height: 75vh;
  overflow-y: scroll;
}
.pdf-viewer-controls {
  margin-top: 10px;
}
</style>