<template>
  <div class="contacts">
    <button class="button is-primary" @click="newContact()">
      Neuer Kontakt
    </button>
    <contact-list ref="contactList" />
  </div>
</template>

<script setup lang="ts">

import ContactForm from "@/components/contacts/ContactFormModal.vue";
import ContactList from "@/components/contacts/ContactList.vue";
import { useProgrammatic } from "@oruga-ui/oruga-next";
import { type Ref, ref } from "vue";

const {oruga} = useProgrammatic();

const contactList: Ref<InstanceType<typeof ContactList> | null> = ref(null)
const newContact = (): void => {
  oruga.modal.open({
    parent: this,
    component: ContactForm,
    hasModalCard: true,
    canCancel: false,
    trapFocus: true,
    events: {
      change: () => {
        contactList.value?.clearCache();
        contactList.value?.reload();
      },
    },
  });
}
</script>
<style scoped lang="scss">
.contacts {
  padding-top: 10px;
}
</style>