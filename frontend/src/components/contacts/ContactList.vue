<template>
  <div class="contact-list" v-if="contacts !== null">
    <o-table
      :data="contacts"
      :backend-pagination="true"
      aria-next-label="Next page"
      aria-previous-label="Previous page"
      aria-page-label="Page"
      aria-current-label="Current page"
    >
      <o-table-column field="id" label="ID" width="20" numeric v-slot="props">
        {{ props.row.id }}
      </o-table-column>
      <o-table-column field="name" label="Name" width="200" v-slot="props">
        {{ props.row.name }}
      </o-table-column>
      <o-table-column custom-key="actions" v-slot="props">
        <button class="button is-small is-light" @click="view(props.row)">
          Öffnen
        </button>
      </o-table-column>
    </o-table>
  </div>
</template>

<script setup lang="ts">

import { type Contact } from "@/models/ContactModel";
import { listContacts } from "@/services/ContactsApiService";
import ContactForm from "./ContactFormModal.vue";
import { ref, type Ref } from "vue";
import { useProgrammatic } from "@oruga-ui/oruga-next";

const { oruga } = useProgrammatic();

let contactsCache: Map<number, Array<Contact>> = new Map();
const contacts: Ref<Array<Contact> | null> = ref(null);
const view = (contact: Contact)  => {
  oruga.modal.open({
    parent: this,
    props: {
      contact: contact,
    },
    component: ContactForm,
    hasModalCard: true,
    canCancel: false,
    trapFocus: true,
    events: {
      change: () => {
        clearCache();
        reload;
      },
    },
  });
}
const clearCache = (): void => {
  contactsCache = new Map();
}
const reload = (): void => {
  pageChange(0);
}
const pageChange = (page: number): void => {
  contacts.value = contactsCache.get(page) ?? null;
  if (contacts.value === null) {
    listContacts(page, 50).then((v) => {
      contacts.value = v;
      contactsCache.set(page, v);
    });
  }
}
listContacts(0, 100000).then((v) => {
    contacts.value = v;
    contactsCache.set(0, v);
  });
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