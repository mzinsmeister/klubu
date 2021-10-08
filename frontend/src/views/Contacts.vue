<template>
  <div class="contacts">
    <button class="button is-primary" @click="newContact()">
      Neuer Kontakt
    </button>
    <contact-list ref="contactList" />
  </div>
</template>

<script lang="ts">
import { Component, Ref, Vue } from "vue-property-decorator";
import ContactList from "@/components/contacts/ContactList.vue";
import ContactForm from "@/components/contacts/ContactFormModal.vue";

@Component({
  components: {
    ContactList,
  },
})
export default class Contacts extends Vue {
  @Ref("contactList") private contactList?: ContactList;

  newContact(): void {
    this.$buefy.modal.open({
      parent: this,
      component: ContactForm,
      hasModalCard: true,
      canCancel: false,
      trapFocus: true,
      events: {
        newContact: () => {
          console.log("t");
          this.contactList?.clearCache();
          this.contactList?.reload();
        },
      },
    });
  }
}
</script>

<style scoped lang="scss">
.contacts {
  padding-top: 10px;
}
</style>
