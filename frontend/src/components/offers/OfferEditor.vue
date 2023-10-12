<template>
  <div class="offer-editor" v-if="offer !== null">
    <div>
      <div class="top-buttons">
        <o-button variant="info" @click="back">Zurück</o-button>
        <o-button variant="success" @click="save">Speichern</o-button>
        <o-button
          v-if="offer.revision !== undefined"
          @click="openRevisionsModal"
          >Revisionen</o-button
        >
        <o-button v-if="!isCommitted" :disabled="offer.id === undefined" variant="danger" @click="commit"
          >Festschreiben</o-button
        >
        <o-button
          v-if="isCommitted && offer.document === undefined"
          variant="warning"
          :loading="isExporting"
          :disabled="offer.id === undefined"
          @click="tryExport"
          >Exportieren</o-button
        >
        <o-button
          v-if="isCommitted && offer.document !== undefined"
          variant="warning"
          tag="a"
          :href="`/api/documents/${offer.document.id}`"
          target="_blank"
          :download="`Angebot ${offer.id}-${offer.revision}.pdf`"
          >PDF Herunterladen</o-button
        >
      </div>
    </div>
    <o-field label="Titel">
      <o-input @update:modelValue="change" v-model="offer.title" />
    </o-field>
    <o-field label="Kunde">
      <contact-search
        :contact="
          offer.customerContact === undefined ? null : offer.customerContact
        "
        :disabled="isCommitted"
        @select="select"
      />
    </o-field>
    <recipient-editor
      v-if="offer.recipient !== undefined"
      @change="change"
      :disabled="isCommitted"
      v-model="offer.recipient"
    />
    <o-field grouped>
      <o-field expanded label="Angebotsdatum">
        <o-datepicker
         @update:modelValue="change"
          :disabled="isCommitted"
          v-model="offer.offerDate"
        />
        <p class="control">
          <o-button
            @click="
              offer.offerDate = undefined;
              change();
            "
            icon-right="delete"
            :disabled="offer.offerDate === undefined || isCommitted"
          />
        </p>
      </o-field>
      <o-field expanded label="Gültig bis">
        <o-datepicker
         @update:modelValue="change"
          :disabled="isCommitted"
          v-model="offer.validUntilDate"
        />
        <p class="control">
          <o-button
            @click="
              offer.validUntilDate = undefined;
              change();
            "
            icon-right="delete"
            :disabled="offer.validUntilDate === undefined || isCommitted"
          />
        </p>
      </o-field>
    </o-field>
    <o-field label="Betreff">
      <o-input
       @update:modelValue="change"
        :disabled="isCommitted"
        v-model="offer.subject"
      />
    </o-field>
    <o-field label="Einleitungstext">
      <o-input
       @update:modelValue="change"
        :disabled="isCommitted"
        type="textarea"
        v-model="offer.headerHTML"
      />
    </o-field>
    <items-editor
      @change="change"
      :disabled="isCommitted"
      v-model="offer.items"
    />
    <p>Gesamt: {{ getTotal() }}</p>
    <o-field label="Fußtext">
      <o-input
        @update:modelValue="change"
        :disabled="isCommitted"
        type="textarea"
        v-model="offer.footerHTML"
      />
    </o-field>
  </div>
</template>

<script setup lang="ts">

import { ref, computed, watch, onMounted, toRef } from "vue";
import {
  commitOffer,
  createOffer,
  createRevision,
  exportOffer,
  fetchOffer,
  fetchOfferNewest,
  updateOffer,
} from "@/services/OffersApiService";
import { type Contact } from "@/models/ContactModel";
import { formatCentsAsMoney } from "@/util/MoneyUtil";
import { type Offer } from "@/models/OfferModel";
import { parseISO } from "date-fns";
import ContactSearch from "../common/ContactSearch.vue";
import ItemsEditor from "../common/ItemsEditor.vue";
import RecipientEditor from "../common/RecipientEditor.vue";
import RevisionsModal from "./RevisionsModal.vue";
import { useRoute, useRouter } from "vue-router";
import { useProgrammatic } from "@oruga-ui/oruga-next";

const { oruga } = useProgrammatic();


const route = useRoute();
const router = useRouter();
const offer = ref<Offer | null>(null);
const changed = ref(false); //TODO: Warn if exporting with unsaved changes
const changedSinceSave = ref(false);
const isExporting = ref(false);
const change = ()  => {
  changedSinceSave.value = true;
}
const isCommitted = computed((): boolean => {
  return offer.value?.committedTimestamp !== undefined;
});
const commit = (): void => {
  if (
    offer.value !== null &&
    offer.value?.id !== undefined &&
    offer.value?.revision !== undefined
  ) {
    commitOffer(offer.value.id, offer.value.revision).then((response) => {
      if (offer.value !== null) {
        offer.value.committedTimestamp = parseISO(response.committedTimestamp);
      }
    });
  }
}
const getTotal = (): string => {
  let total = 0;
  if (offer.value !== null) {
    offer.value.items.forEach((item) => {
      total += Number.parseInt(
        (item.price.amountCents * item.quantity).toFixed(0)
      );
    });
  }
  return formatCentsAsMoney(total);
}
const tryExport = ()  => {
  if (changedSinceSave.value) {
    oruga.dialog.confirm({
      message:
        "Das Angebot enthält ungespeicherte Änderungen!\n" +
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
}
const exportDocument = ()  => {
  if (offer.value !== null) {
    const startOfferId = offer.value.id;
    const startOfferRevision = offer.value.revision;
    exportOffer(offer.value)
      .then((r) => {
        if (
          offer.value !== null &&
          offer.value?.id === startOfferId &&
          offer.value?.revision === startOfferRevision
        ) {
          offer.value.document = r.document;
        }
        isExporting.value = false;
        oruga.notification.open({
          message: "Export erfolgreich",
          variant: "success",
          position: "bottom-right"
        });
      })
      .catch(() => {
        isExporting.value = false;
        oruga.notification.open({
          message: "Fehler beim Export",
          variant: "danger",
          position: "bottom-right"
        });
      });
    isExporting.value = true;
  }
}
const back = ()  => {
  router.push({
    path: "/offers",
    query: { forceRefresh: changed.value.toString() },
  });
}
const select = (option: Contact)  => {
  if (offer.value !== null) {
    offer.value.customerContact = option;
    offer.value.recipient!.formOfAddress = option.formOfAddress;
    offer.value.recipient!.title = option.title;
    offer.value.recipient!.name = option.name;
    offer.value.recipient!.firstName = option.firstName;
    offer.value.recipient!.street = option.street;
    offer.value.recipient!.zipCode = option.zipCode;
    offer.value.recipient!.city = option.city;
    offer.value.recipient!.houseNumber = option.houseNumber;
    offer.value.recipient!.country = option.country;
  }
  change();
}
const save = (): void => {
  if (offer.value !== null && offer.value?.id === undefined) {
    createOffer(offer.value).then((result) => {
      history.replaceState(
        history.state,
        document.title,
        "/offers/" + result.id
      );
      offer.value = result;
      changedSinceSave.value = false;
    });
  } else if (offer.value !== null) {
    updateOffer(offer.value).then(() => (changedSinceSave.value = false));
  }
  changed.value = true;
}

const fetchCurrentOffer = ()  => {
  const id = route.params["id"] as string;
  if (id === "new") {
    offer.value = {
      items: [],
      subject: "Angebot",
      recipient: { name: "" },
    };
  } else if (route.params["revision"] === undefined) {
    fetchOfferNewest(Number.parseInt(id)).then((v) => {
      if (v.recipient === undefined) {
        v.recipient = { name: "" };
      }
      offer.value = v;
    });
  } else {
    const revision = route.params["revision"] as string;
    fetchOffer(Number.parseInt(id), Number.parseInt(revision)).then((v) => {
      if (v.recipient === undefined) {
        v.recipient = { name: "" };
      }
      offer.value = v;
    });
  }
}

onMounted(() => {
    fetchCurrentOffer();
  });

const onUrlChange = ()  => {
  fetchCurrentOffer();
}
watch(route, () => onUrlChange(), { immediate: true, deep: true });


const openRevisionsModal = ()  => {
  if (offer.value?.id !== undefined) {
    oruga.modal.open({
      parent: this,
      props: {
        offerId: offer.value.id,
      },
      component: RevisionsModal,
      hasModalCard: true,
      canCancel: false,
      trapFocus: true,
      events: {
        createRevision: () => {
          if (offer.value !== null && offer.value.id !== undefined) {
            createRevision(offer.value).then((o) => {
              router.push(`/offers/${o.id}/revisions/${o.revision}`);
            });
          }
        },
      },
    });
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