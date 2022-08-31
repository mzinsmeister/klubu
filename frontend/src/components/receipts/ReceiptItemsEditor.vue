<template>
  <div class="items-editor">
    <div class="receipt-item-inputs" v-for="(item, i) in value" :key="i">
      <b-field label="Beschreibung">
        <b-input
          @input="change"
          :disabled="isDisabled"
          class="position-input"
          v-model="item.item"
        />
      </b-field>
      <b-field label="Kategorie">
        <b-select 
          @input="change"
          :disabled="isDisabled"
          placeholder="Kategorie wählen"
          class="position-input"
          style="min-width: 200px"
          v-model="item.category"
          :loading="item.category === undefined && itemCategories === null"
          >
            <option
                v-for="option in itemCategories"
                :value="option"
                :key="option.id">
                {{ option.name }}
            </option>
        </b-select>
      </b-field>
      <b-field grouped group-multiline>
        <b-field label="Preis in Cent">
          <b-input
            @input="change"
            :disabled="isDisabled"
            class="position-input"
            v-model="item.price.amountCents"
          />
        </b-field>
        <b-field label="Löschen?">
          <b-button
            :disabled="isDisabled"
            icon-right="delete"
            type="is-danger"
            @click="deleteItem(i)"
          />
      </b-field>
    </b-field>  
    </div>
    <b-button @click="addEmptyItem()" style="margin-top: 20px" :disabled="isDisabled"
      >Zusätzliche Position</b-button
    >
  </div>
</template>

<script lang="ts">
import { ReceiptItem, ReceiptItemCategory } from "@/models/ReceiptModel";
import { fetchReceiptItemCategories } from "@/services/ReceiptsApiService";
import { formatCentsAsMoney } from "@/util/MoneyUtil";
import { Component, Prop, Vue } from "vue-property-decorator";

//TODO: amountCents is String after input but should be number (somehow still works)
@Component({
  name: "items-editor",
})
export default class ItemsEditor extends Vue {
  @Prop() private value!: ReceiptItem[];
  @Prop({ required: false }) private disabled?: boolean;

  private itemCategories: Array<ReceiptItemCategory> | null = null

  private created() {
    if (this.value.length === 0) {
      this.addEmptyItem();
    }
    fetchReceiptItemCategories().then((data) => {
      this.itemCategories = data;
    })
  }

  private get isDisabled(): boolean {
    return this.disabled !== undefined ? this.disabled : false;
  }

  private deleteItem(index: number) {
    this.value.splice(index, 1);
    this.change();
  }

  private formatCentsAsMoney(cents: number): string {
    return formatCentsAsMoney(cents);
  }

  private addEmptyItem(): void {
    const newItem: ReceiptItem = {
      item: "",
      price: { amountCents: 0, currency: { code: "EUR" } },
      category: undefined
    };
    this.value.push(newItem);
    this.change();
  }

  private change(): void {
    this.$emit("change");
  }
}
</script>

<style scoped lang="scss">
  .receipt-item-inputs {
    margin-top: 20px;
    border-bottom: 1px solid lightgrey;
    padding-bottom: 20px;
  }
</style>
