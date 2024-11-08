import { ConnectError, createPromiseClient } from "@connectrpc/connect";
import { createGrpcWebTransport } from "@connectrpc/connect-web";
import test, { after, before, describe } from "node:test";
import assert from "node:assert";
import _ from "lodash";
import "dotenv/config";

import { CommerceService } from "../api/sited_io/commerce/v2/commerce_service_connect";
import {
  Offer_Details,
  OfferType,
  OfferType_Digital,
  OfferType_Physical,
} from "../api/sited_io/commerce/v2/offer_pb";
import { Auth } from "../common/auth";
import {
  OffersFilterField,
  OffersOrderByField,
} from "../api/sited_io/commerce/v2/query_pb";
import { Direction } from "../api/sited_io/query/v1/query_pb";

const baseUrl = process.env.BASE_URL!;
const authUrl = process.env.AUTH_URL!;
const tester1ClientId = process.env.TESTER_1_CLIENT_ID!;
const tester1ClientSecret = process.env.TESTER_1_CLIENT_SECRET!;
const tester1UserId = process.env.TESTER_1_USER_ID!;

const testOffer1 = {
  offerId: "",
  details: {
    name: "Test Offer1",
  } as Offer_Details,
  offerType: {
    offerTypeKind: {
      value: new OfferType_Physical(),
      case: "physical",
    },
  } as OfferType,
};
const testOffer2 = {
  offerId: "",
  details: {
    name: "Test Offer2",
  } as Offer_Details,
  offerType: {
    offerTypeKind: {
      value: new OfferType_Digital(),
      case: "digital",
    },
  } as OfferType,
};

if (
  _.isNil(
    baseUrl ||
      authUrl ||
      tester1ClientId ||
      tester1ClientSecret ||
      tester1UserId
  )
) {
  throw new Error("Please provide environment variable 'TEST_BASE_URL'");
}

const transport = createGrpcWebTransport({ baseUrl });
const commerceClient = createPromiseClient(CommerceService, transport);

const auth1 = new Auth(authUrl, tester1ClientId, tester1ClientSecret);

async function cleanup() {
  const offers = await commerceClient.listOffers({
    owner: tester1UserId,
  });
  for (const { offerId } of offers.offers) {
    await commerceClient.deleteOffer(
      {
        offerId,
      },
      await auth1.withAuthHeader()
    );
  }
}

describe("Offers", () => {
  before(async () => {
    await cleanup();
  });
  after(async () => {
    await cleanup();
  });

  test("Create Offer : empty request : nok", async () => {
    let offer;
    try {
      offer = await commerceClient.createOffer({});
    } catch (err) {
      assert(!_.isNil(err));
      assert((err as ConnectError).code === 16);
    }
    assert(_.isNil(offer));
  });

  test("Create Offer : unauthenticated : nok", async () => {
    let offer;
    try {
      offer = await commerceClient.createOffer({
        details: {
          name: "Test Offer : unauthenticated : shold not exist",
        },
        offerType: {
          offerTypeKind: {
            value: new OfferType_Physical(),
            case: "physical",
          },
        },
      });
    } catch (err) {
      assert(!_.isNil(err));
      assert((err as ConnectError).code === 16);
    }
    assert(_.isNil(offer));
  });

  test("Create Offer : missing details : nok", async () => {
    let offer;
    try {
      offer = await commerceClient.createOffer(
        {
          offerType: {
            offerTypeKind: {
              value: new OfferType_Physical(),
              case: "physical",
            },
          },
        },
        await auth1.withAuthHeader()
      );
    } catch (err) {
      assert(!_.isNil(err));
      assert((err as ConnectError).code === 3);
    }
    assert(_.isNil(offer));
  });

  test("Create Offer : missing offerType : nok", async () => {
    let offer;
    try {
      offer = await commerceClient.createOffer(
        {
          details: {
            name: "Test Offer : missing offerType : shold not exist",
          },
        },
        await auth1.withAuthHeader()
      );
    } catch (err) {
      assert(!_.isNil(err));
      assert((err as ConnectError).code === 3);
    }
    assert(_.isNil(offer));
  });

  test("Create Offer: physical : ok", async () => {
    const { offer } = await commerceClient.createOffer(
      {
        details: testOffer1.details,
        offerType: testOffer1.offerType,
      },
      await auth1.withAuthHeader()
    );

    assert(!_.isNil(offer));
    assert(!_.isNil(offer.details));
    assert(offer.details.name === testOffer1.details.name);

    testOffer1.offerId = offer.offerId;
  });

  test("Create Offer: digital : ok", async () => {
    const { offer } = await commerceClient.createOffer(
      {
        details: testOffer2.details,
        offerType: testOffer2.offerType,
      },
      await auth1.withAuthHeader()
    );

    assert(!_.isNil(offer));
    assert(!_.isNil(offer.details));
    assert(offer.details.name === testOffer2.details.name);

    testOffer2.offerId = offer.offerId;
  });

  test("Get Offer : empty request : nok", async () => {
    let offer;
    try {
      offer = await commerceClient.getOffer({});
    } catch (err) {
      assert(!_.isNil(err));
      assert((err as ConnectError).code === 3);
    }
    assert(_.isNil(offer));
  });

  test("Get Offer : empty offerId : nok", async () => {
    let offer;
    try {
      offer = await commerceClient.getOffer({ offerId: "" });
    } catch (err) {
      assert(!_.isNil(err));
      assert((err as ConnectError).code === 3);
    }
    assert(_.isNil(offer));
  });

  test("Get Offer : ok", async () => {
    const { offer } = await commerceClient.getOffer({
      offerId: testOffer1.offerId,
    });

    assert(!_.isNil(offer));
    assert(!_.isNil(offer.details));
    assert(offer.details.name === testOffer1.details.name);
    assert(_.isNil(offer.details.description));
  });

  test("List Offers : empty request : ok", async () => {
    const { offers, pagination } = await commerceClient.listOffers({});

    assert(!_.isNil(offers));
    assert(!_.isEmpty(offers));
    assert(!_.isNil(pagination));
  });

  test("List Offers : owner : ok", async () => {
    const { offers, pagination } = await commerceClient.listOffers({
      owner: tester1UserId,
    });

    assert(!_.isNil(offers));
    assert(!_.isEmpty(offers));
    assert(!_.isNil(pagination));
  });

  test("List Offers : shopId : ok", async () => {
    const { offers, pagination } = await commerceClient.listOffers({
      shopId: "42e64f30-9708-4bcc-ae6d-7ecfe69a80e4",
    });

    assert(!_.isNil(offers));
    assert(_.isEmpty(offers));
    assert(!_.isNil(pagination));
  });

  test("List Offers : pagination page 0 : nok", async () => {
    let res;
    try {
      res = await commerceClient.listOffers({
        pagination: {
          page: 0,
          size: 10,
        },
      });
    } catch (err) {
      assert(!_.isNil(err));
      assert((err as ConnectError).code === 3);
    }
    assert(_.isNil(res));
  });

  test("List Offers : pagination : ok", async () => {
    const { offers, pagination } = await commerceClient.listOffers({
      pagination: {
        page: 1,
        size: 1,
      },
    });

    assert(!_.isNil(offers));
    assert(offers.length <= 1);
    assert(!_.isNil(pagination));
  });

  test("List Offers : pagination : ok", async () => {
    const { offers, pagination } = await commerceClient.listOffers({
      pagination: {
        page: 1,
        size: 1,
      },
    });

    assert(!_.isNil(offers));
    assert(offers.length <= 1);
    assert(!_.isNil(pagination));
  });

  test("List Offers : order by : ok", async () => {
    let res = await commerceClient.listOffers({
      orderBy: {
        field: OffersOrderByField.CREATED_AT,
        direction: Direction.ASC,
      },
    });

    assert(!_.isNil(res.offers));
    assert(!_.isNil(res.pagination));
    assert(res.offers[0].details?.name == testOffer1.details.name);

    res = await commerceClient.listOffers({
      orderBy: {
        field: OffersOrderByField.CREATED_AT,
        direction: Direction.DESC,
      },
    });

    assert(!_.isNil(res.offers));
    assert(!_.isNil(res.pagination));
    assert(res.offers[0].details?.name == testOffer2.details.name);
  });

  test("List Offers : filter : ok", async () => {
    const { offers, pagination } = await commerceClient.listOffers({
      filter: {
        field: OffersFilterField.NAME,
        query: testOffer1.details.name,
      },
    });

    assert(!_.isNil(offers));
    assert(!_.isNil(pagination));
    assert(offers.length == 1);
    assert(offers[0].details?.name === testOffer1.details.name);
  });

  test("Delete Offer", async () => {
    await commerceClient.deleteOffer(
      { offerId: testOffer1.offerId },
      await auth1.withAuthHeader()
    );
  });
});
