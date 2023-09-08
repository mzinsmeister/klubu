<template>
  <div class="props.contact-search">
    <o-autocomplete
      :disabled="isDisabled"
      :data="contactSuggestions"
      v-model="contactString"
      @typing="getcontactSuggestions"
      @select="select"
      :clear-on-select="true"
    >
      <template v-slot="option">
        <div class="customerSuggestion">
          {{ formatContact(option) }}
        </div>
      </template>
    </o-autocomplete>
  </div>
</template>

<script setup lang="ts">
import { OAutocomplete } from "@oruga-ui/oruga-next"
import { ref, computed } from "vue";
import type { Contact } from "@/models/ContactModel";
import { listContacts } from "@/services/ContactsApiService";

const formatContact = (contact: Contact): string => {
    let result = contact.name;
    if (
      contact.firstName !== undefined &&
      contact.firstName !== null &&
      contact.firstName.length > 0
    ) {
      result += ", " + contact.firstName;
    }
    return result;
  }

const { contact, disabled } = defineProps<{
  contact?:  Contact | null, 
  disabled?: boolean
}>();

const emit = defineEmits(["select", "change"]);

const contactSuggestions = ref<Contact[]>([]);
let contactString = contact ? formatContact(contact) : "";
const isDisabled = computed((): boolean => {
  return disabled !== undefined ? disabled : false;
});

const getcontactSuggestions = (name: string): void => {
  listContacts(0, 10, name).then((v) => (contactSuggestions.value = v));
}
const select = (option: Contact)  => {
  contactString = formatContact(option);
  emit("select", option);
}
</script>
<style scoped lang="scss"></style>