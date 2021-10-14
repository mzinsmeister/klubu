<template>
  <div class="contact-list" v-if="contacts !== null">
    <b-table
      :data="contacts"
      :backend-pagination="true"
      aria-next-label="Next page"
      aria-previous-label="Previous page"
      aria-page-label="Page"
      aria-current-label="Current page"
    >
      <b-table-column field="id" label="ID" width="20" numeric v-slot="props">
        {{ props.row.id }}
      </b-table-column>
      <b-table-column field="name" label="Name" width="200" v-slot="props">
        {{ props.row.name }}
      </b-table-column>
      <b-table-column custom-key="actions" v-slot="props">
        <button class="button is-small is-light" @click="view(props.row)">
          Öffnen
        </button>
      </b-table-column>
    </b-table>
  </div>
</template>

<script lang="ts">
import { Contact } from "@/models/ContactModel";
import { listContacts } from "@/services/ContactsApiService";
import { Component, Vue } from "vue-property-decorator";
import ContactForm from "./ContactFormModal.vue";

@Component
export default class ContactList extends Vue {
  private contactsCache: Map<number, Array<Contact>> = new Map();
  private contacts: Array<Contact> | null = null;

  private created(): void {
    listContacts(0, 100000).then((v) => {
      this.contacts = v;
      this.contactsCache.set(0, v);
    });
  }

  private view(contact: Contact) {
    this.$buefy.modal.open({
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
          this.clearCache();
          this.reload;
        },
      },
    });
  }

  clearCache(): void {
    this.contactsCache = new Map();
  }

  reload(): void {
    this.pageChange(0);
  }

  private pageChange(page: number): void {
    this.contacts = this.contactsCache.get(page) ?? null;
    if (this.contacts === null) {
      listContacts(page, 50).then((v) => {
        this.contacts = v;
        this.contactsCache.set(page, v);
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
