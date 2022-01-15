<template>
  <div class="receipt-editor" v-if="receipt !== null">
    <div>
      <div class="top-buttons">
        <b-button type="is-info" @click="back">Zurück</b-button>
        <b-button type="is-success" @click="save">Speichern</b-button>
        <b-button
          :disabled="isCommitted || receipt.id === undefined"
          type="is-danger"
          @click="commit"
          >Festschreiben</b-button
        >
      </div>
    </div>
    <div class="columns is-vcentered">
      <div class="column is-8">
        <div v-if="receipt.documentData !== null || pdfSrc !== null">
          <div class="pdf-viewer">
            <pdf v-for="i in pdfNumPages" :key="i" :page="i" :src="pdfSrc" />
          </div>
          <div class="pdf-viewer-controls">
            <b-button type="is-danger" @click="removeDocument"
              >Dokument entfernen</b-button
            >
          </div>
        </div>
        <b-upload
          v-if="receipt.documentData === null && pdfSrc == null"
          v-model="fileUpload"
          accept=".pdf"
          drag-drop
          @input="changeFile"
        >
          <section class="section">
            <div class="content has-text-centered">
              <p>
                <b-icon icon="upload" size="is-large"> </b-icon>
              </p>
              <p>Datei hier hin ziehen oder klicken um Datei auszuwählen</p>
            </div>
          </section>
        </b-upload>
      </div>
      <div class="column inputcolumn">
        <b-field label="Belegnummer">
          <b-input v-model="receipt.receiptNumber" :disabled="isCommitted" />
        </b-field>
        <b-field label="Lieferant">
          <contact-search
            :contact="
              receipt.supplierContact === undefined
                ? null
                : receipt.supplierContact
            "
            :disabled="isCommitted"
            @select="select"
          />
        </b-field>
        <b-field expanded label="Belegdatum">
          <b-datepicker
            @input="change"
            :disabled="isCommitted"
            v-model="receipt.receiptDate"
            expanded
          />
          <p class="control">
            <b-button
              @click="
                receipt.receiptDate = undefined;
                change();
              "
              icon-right="delete"
              :disabled="receipt.receiptDate === undefined || isCommitted"
            />
          </p>
        </b-field>
        <b-field label="Bezahlt am">
          <b-datepicker
            @input="change"
            v-model="receipt.paidDate"
            :disabled="isCommitted"
            expanded
          />
          <p class="control">
            <b-button
              @click="
                receipt.paidDate = undefined;
                change();
              "
              icon-right="delete"
              :disabled="receipt.paidDate === undefined || isCommitted"
            />
          </p>
        </b-field>
        <b-field label="Zu bezahlen bis">
          <b-datepicker
            @input="change"
            v-model="receipt.dueDate"
            :disabled="isCommitted"
            expanded
          />
          <p class="control">
            <b-button
              @click="
                receipt.dueDate = undefined;
                change();
              "
              icon-right="delete"
              :disabled="receipt.dueDate === undefined || isCommitted"
            />
          </p>
        </b-field>
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

<script lang="ts">
import { Contact } from "@/models/ContactModel";
import { Receipt } from "@/models/ReceiptModel";
import ContactSearch from "../common/ContactSearch.vue";
import ReceiptItemsEditor from "./ReceiptItemsEditor.vue";
import {
  commitReceipt,
  createReceipt,
  fetchReceipt,
  updateReceipt,
} from "@/services/ReceiptsApiService";
import { formatCentsAsMoney } from "@/util/MoneyUtil";
import { Component, Vue } from "vue-property-decorator";
import { parseISO } from "date-fns";
import pdf from "vue-pdf"; // TODO: Causes lots of errors but they don't seem to be a problem
import { PDFDocumentLoadingTask } from "pdfjs-dist/types/display/api";

@Component({
  name: "receipt-editor",
  components: {
    ContactSearch,
    ReceiptItemsEditor,
    pdf,
  },
})
export default class ReceiptEditor extends Vue {
  private receipt: Receipt | null = null;
  private changed = false;
  private changedSinceSave = false;
  private fileUpload: File | null = null;
  private pdfNumPages = 0;
  private pdfSrc: PDFDocumentLoadingTask | null = null;
  private documentChanged = false;

  private change() {
    this.changedSinceSave = true;
  }

  private get isCommitted(): boolean {
    return this.receipt?.committedTimestamp !== undefined;
  }

  private changeFile(file: File) {
    file.arrayBuffer().then((b) => {
      if (this.receipt !== null) {
        this.receipt.documentData = {
          data: new Uint8Array(b),
          mediaType: "application/pdf",
        };
        this.pdfSrc = pdf.createLoadingTask(this.receipt.documentData.data);
        this.pdfSrc!.promise.then((r) => {
          this.pdfNumPages = r.numPages;
        });
      }
      this.documentChanged = true;
    });
    this.change();
  }

  private removeDocument() {
    if (this.receipt !== null) {
      this.receipt.documentData = null;
      this.pdfSrc = null;
      this.documentChanged = true;
      this.change();
    }
  }

  private commit(): void {
    if (this.receipt !== null && this.receipt?.id !== undefined) {
      commitReceipt(this.receipt.id).then((response) => {
        if (this.receipt !== null) {
          this.receipt.committedTimestamp = parseISO(
            response.committedTimestamp
          );
        }
      });
    }
  }

  private getTotal(): string {
    let total = 0;
    if (this.receipt !== null) {
      this.receipt.items.forEach((item) => {
        total += Number.parseInt(item.price.amountCents.toString());
      });
    }
    return this.formatCentsAsMoney(total);
  }

  private formatCentsAsMoney(cents: number): string {
    return formatCentsAsMoney(cents);
  }

  private back() {
    this.$router.push({
      path: "/receipts",
      query: { forceRefresh: this.changed.toString() },
    });
  }

  private select(option: Contact) {
    if (this.receipt !== null) {
      this.receipt.supplierContact = option;
    }
    this.change();
  }

  private save(): void {
    if (this.receipt !== null && this.receipt?.id === undefined) {
      createReceipt(this.receipt).then((result) => {
        history.replaceState(
          history.state,
          document.title,
          "/receipts/" + result.id
        );
        this.receipt = result;
        this.changedSinceSave = false;
        this.documentChanged = false;
      });
    } else if (this.receipt !== null) {
      updateReceipt(this.receipt, this.documentChanged).then(() => {
        this.changedSinceSave = false;
        this.documentChanged = false;
      });
    }
    this.changed = true;
  }

  private created(): void {
    const id = this.$route.params["id"];
    if (id === "new") {
      this.receipt = {
        items: [],
        receiptNumber: "",
        documentData: null,
      };
    } else {
      fetchReceipt(Number.parseInt(id)).then((v) => {
        this.receipt = v;
        if (v.document !== undefined) {
          this.pdfSrc = pdf.createLoadingTask(
            `/api/documents/${v.document.id}`
          );
          this.pdfSrc!.promise.then((r) => {
            this.pdfNumPages = r.numPages;
          });
        }
      });
    }
  }
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
