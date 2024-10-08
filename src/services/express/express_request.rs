#![doc = include_str!("../../../docs/client/express.md")]

use chrono::prelude::Local;
use chrono::DateTime;
use derive_builder::Builder;
use openssl::base64;
use serde::{Deserialize, Serialize};
use url::Url;

use super::{serialize_utc_to_string, DEFAULT_PASSKEY};
use crate::client::Mpesa;
use crate::constants::CommandId;
use crate::errors::{MpesaError, MpesaResult};
use crate::validator::PhoneNumberValidator;
const EXPRESS_REQUEST_URL: &str = "mpesa/stkpush/v1/processrequest";

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MpesaExpressRequest<'mpesa> {
    /// This is the organization's shortcode (Paybill or Buygoods - A 5 to
    /// 6-digit account number) used to identify an organization and receive
    /// the transaction.
    pub business_short_code: &'mpesa str,
    /// This is the password used for encrypting the request sent:
    pub password: String,
    /// This is the Timestamp of the transaction, normally in the format of
    /// (YYYYMMDDHHMMSS)
    #[serde(serialize_with = "serialize_utc_to_string")]
    pub timestamp: DateTime<Local>,
    /// This is the transaction type that is used to identify the transaction
    /// when sending the request to M-PESA
    ///
    /// The TransactionType for Mpesa Express is either
    /// `CommandId::BusinessBuyGoods` or
    /// `CommandId::CustomerPayBillOnline`
    pub transaction_type: CommandId,
    /// This is the Amount transacted normally a numeric value
    pub amount: u32,
    ///The phone number sending money.
    pub party_a: &'mpesa str,
    /// The organization that receives the funds
    pub party_b: &'mpesa str,
    /// The Mobile Number to receive the STK Pin Prompt.
    /// This number can be the same as PartyA value above.
    ///
    ///  The parameter expected is a Valid Safaricom Mobile Number that is
    /// M-PESA registered in the format 2547XXXXXXXX
    pub phone_number: &'mpesa str,
    /// A CallBack URL is a valid secure URL that is used to receive
    /// notifications from M-Pesa API.
    /// It is the endpoint to which the results will be sent by M-Pesa API.
    #[serde(rename = "CallBackURL")]
    pub call_back_url: Url,
    /// Account Reference: This is an Alpha-Numeric parameter that is defined
    /// by your system as an Identifier of the transaction for
    /// CustomerPayBillOnline
    pub account_reference: &'mpesa str,
    /// This is any additional information/comment that can be sent along with
    /// the request from your system
    pub transaction_desc: Option<&'mpesa str>,
}

// TODO:: The success response has more fields than this
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MpesaExpressResponse {
    ///This is a global unique identifier of the processed checkout transaction
    /// request.
    #[serde(rename = "CheckoutRequestID")]
    pub checkout_request_id: String,
    /// This is a message that your system can display to the customer as an
    /// acknowledgment of the payment request submission.
    pub customer_message: String,
    /// This is a global unique Identifier for any submitted payment request.
    #[serde(rename = "MerchantRequestID")]
    pub merchant_request_id: String,
    /// This is a Numeric status code that indicates the status of the
    /// transaction submission. 0 means successful submission and any other
    /// code means an error occurred.
    pub response_code: String,
    ///Response description is an acknowledgment message from the API that
    /// gives the status of the request submission. It usually maps to a
    /// specific ResponseCode value.
    ///
    /// It can be a Success submission message or an error description.
    pub response_description: String,
}

#[derive(Builder, Debug, Clone)]
#[builder(build_fn(error = "MpesaError", validate = "Self::validate"))]
pub struct MpesaExpress<'mpesa> {
    #[builder(pattern = "immutable")]
    client: &'mpesa Mpesa,
    /// This is the organization's shortcode (Paybill or Buygoods - A 5 to
    /// 6-digit account number) used to identify an organization and receive
    /// the transaction.
    #[builder(setter(into))]
    business_short_code: &'mpesa str,
    /// This is the transaction type that is used to identify the transaction
    /// when sending the request to M-PESA
    ///
    /// The TransactionType for Mpesa Express is either
    /// `CommandId::BusinessBuyGoods` or
    /// `CommandId::CustomerPayBillOnline`
    transaction_type: CommandId,
    /// This is the Amount transacted normally a numeric value
    amount: u32,
    /// The phone number sending money.
    party_a: &'mpesa str,
    /// The organization that receives the funds
    party_b: &'mpesa str,
    /// The Mobile Number to receive the STK Pin Prompt.
    phone_number: &'mpesa str,
    /// A CallBack URL is a valid secure URL that is used to receive
    /// notifications from M-Pesa API.
    /// It is the endpoint to which the results will be sent by M-Pesa API.
    #[builder(try_setter, setter(into))]
    callback_url: Url,
    /// Account Reference: This is an Alpha-Numeric parameter that is defined
    /// by your system as an Identifier of the transaction for
    /// CustomerPayBillOnline
    #[builder(setter(into))]
    account_ref: &'mpesa str,
    /// This is any additional information/comment that can be sent along with
    /// the request from your system
    #[builder(setter(into, strip_option), default)]
    transaction_desc: Option<&'mpesa str>,
    /// This is the password used for encrypting the request sent:
    /// The password for encrypting the request is obtained by base64 encoding
    /// BusinessShortCode, Passkey and Timestamp.
    /// The timestamp format is YYYYMMDDHHmmss
    #[builder(setter(into, strip_option), default = "Some(DEFAULT_PASSKEY)")]
    pass_key: Option<&'mpesa str>,
}

impl<'mpesa> From<MpesaExpress<'mpesa>> for MpesaExpressRequest<'mpesa> {
    fn from(express: MpesaExpress<'mpesa>) -> MpesaExpressRequest<'mpesa> {
        let timestamp = chrono::Local::now();

        let encoded_password =
            MpesaExpress::encode_password(express.business_short_code, express.pass_key);

        MpesaExpressRequest {
            business_short_code: express.business_short_code,
            password: encoded_password,
            timestamp,
            transaction_type: express.transaction_type,
            amount: express.amount,
            party_a: express.party_a,
            party_b: express.party_b,
            phone_number: express.phone_number,
            call_back_url: express.callback_url,
            account_reference: express.account_ref,
            transaction_desc: express.transaction_desc,
        }
    }
}

impl MpesaExpressBuilder<'_> {
    /// Validates the request, returning a `MpesaError` if validation fails
    ///
    /// Express requests can only be of type `BusinessBuyGoods` or
    /// `CustomerPayBillOnline`
    fn validate(&self) -> MpesaResult<()> {
        if self.transaction_type != Some(CommandId::BusinessBuyGoods)
            && self.transaction_type != Some(CommandId::CustomerPayBillOnline)
        {
            return Err(MpesaError::Message(
                "Invalid transaction type. Expected BusinessBuyGoods or CustomerPayBillOnline",
            ));
        }

        if let Some(phone_number) = self.phone_number {
            phone_number.validate()?;
        }

        Ok(())
    }
}

impl<'mpesa> MpesaExpress<'mpesa> {
    /// Creates new `MpesaExpressBuilder`
    pub(crate) fn builder(client: &'mpesa Mpesa) -> MpesaExpressBuilder<'mpesa> {
        MpesaExpressBuilder::default().client(client)
    }

    /// Encodes the password for the request
    /// The password for encrypting the request is obtained by base64 encoding
    /// BusinessShortCode, Passkey and Timestamp.
    /// The timestamp format is YYYYMMDDHHmmss
    pub fn encode_password(business_short_code: &str, pass_key: Option<&'mpesa str>) -> String {
        let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        base64::encode_block(
            format!(
                "{}{}{}",
                business_short_code,
                pass_key.unwrap_or(DEFAULT_PASSKEY),
                timestamp
            )
            .as_bytes(),
        )
    }

    /// Creates a new `MpesaExpress` from a `MpesaExpressRequest`
    pub fn from_request(
        client: &'mpesa Mpesa,
        request: MpesaExpressRequest<'mpesa>,
        pass_key: Option<&'mpesa str>,
    ) -> MpesaExpress<'mpesa> {
        MpesaExpress {
            client,
            business_short_code: request.business_short_code,
            transaction_type: request.transaction_type,
            amount: request.amount,
            party_a: request.party_a,
            party_b: request.party_b,
            phone_number: request.phone_number,
            callback_url: request.call_back_url,
            account_ref: request.account_reference,
            transaction_desc: request.transaction_desc,
            pass_key,
        }
    }

    /// # Lipa na M-Pesa Online Payment / Mpesa Express/ Stk push
    ///
    /// Initiates a M-Pesa transaction on behalf of a customer using STK Push
    ///
    /// A successful request returns a `MpesaExpressRequestResponse` type
    ///
    /// # Errors
    /// Returns a `MpesaError` on failure
    pub async fn send(self) -> MpesaResult<MpesaExpressResponse> {
        self.client
            .send::<MpesaExpressRequest, _>(crate::client::Request {
                method: reqwest::Method::POST,
                path: EXPRESS_REQUEST_URL,
                body: self.into(),
            })
            .await
    }
}
