export interface Money {
  amountCents: number;
  currency: Currency;
}

export interface Currency {
  code: string;
  symbol?: string;
}

export interface Recipent {
  formOfAddress?: string;
  title?: string;
  name: string;
  firstName?: string;
  street?: string;
  zipCode?: string;
  city?: string;
  houseNumber?: string;
  country?: string;
}