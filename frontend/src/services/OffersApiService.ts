import {
  ApiPage,
  OfferListItemDTO,
  RequestOfferDTO,
  ResponseOfferDTO,
} from "@/models/ApiModel";
import { Offer, OfferListItem } from "@/models/OfferModel";
import { formatISO, parseISO } from "date-fns";
import Vue from "vue";

export async function listOffers(
  page: number,
  pageSize: number
): Promise<Array<OfferListItem>> {
  const response = await Vue.axios.get<ApiPage<OfferListItemDTO>>(
    "/api/offers",
    {
      params: {
        page: page,
        size: pageSize,
      },
    }
  );
  return response.data.content.map((dto) => ({
    id: dto.id,
    revision: dto.revision,
    title: dto.title,
    createdTimestamp: parseISO(dto.createdTimestamp),
    customerContact: dto.customerContact,
  }));
}

function mapOfferDTOToOffer(dto: ResponseOfferDTO): Offer {
  return {
    id: dto.id,
    revision: dto.revision,
    title: dto.title,
    customerContact: dto.customerContact,
    recipient: dto.recipient,
    items: dto.items,
    createdTimestamp: parseISO(dto.createdTimestamp),
    offerDate: dto.offerDate ? parseISO(dto.offerDate) : undefined,
    validUntilDate: dto.validUntilDate
      ? parseISO(dto.validUntilDate)
      : undefined,
    subject: dto.subject,
    headerHTML: dto.headerHTML,
    footerHTML: dto.footerHTML,
  };
}

export async function fetchOffer(id: number): Promise<Offer> {
  const response = await Vue.axios.get<ResponseOfferDTO>("/api/offers/" + id);
  return mapOfferDTOToOffer(response.data);
}

function mapOfferToDTO(offer: Offer): RequestOfferDTO {
  return {
    customerContactId: offer.customerContact?.id,
    title: offer.title,
    items: offer.items,
    subject: offer.subject,
    validUntilDate: offer.validUntilDate
      ? formatISO(offer.validUntilDate, { representation: "date" })
      : undefined,
    offerDate: offer.offerDate
      ? formatISO(offer.offerDate, { representation: "date" })
      : undefined,
    footerHTML: offer.footerHTML,
    headerHTML: offer.headerHTML,
    recipient: offer.recipient,
  };
}

export async function createOffer(offer: Offer): Promise<Offer> {
  const response = await Vue.axios.post("/api/offers", mapOfferToDTO(offer));
  return mapOfferDTOToOffer(response.data);
}

export async function updateOffer(offer: Offer): Promise<void> {
  await Vue.axios.put(
    `/api/offers/${offer.id}/revisions/${offer.revision}`,
    mapOfferToDTO(offer)
  );
}

export async function exportOffer(offer: Offer): Promise<void> {
  await Vue.axios.post(
    `/api/offers/${offer.id}/revisions/${offer.revision}/export`
  );
}
