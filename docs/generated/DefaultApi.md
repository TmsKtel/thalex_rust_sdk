# \DefaultApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_ticker**](DefaultApi.md#get_ticker) | **GET** /ticker/{instrument}/{delay} | Get ticker feed



## get_ticker

> models::TickerResponse get_ticker(instrument, delay)
Get ticker feed

Get real-time ticker data for a single instrument or combination.  For combinations, mark price is the quantity-weighted sum of mark prices of leg instruments. Best bid/ask for combinations represent top levels of a virtual combination order book. 

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**instrument** | **String** | For single instruments: instrument name (e.g. \"BTC-PERPETUAL\") For combinations: format [<instrument_name>:<quantity>,<instrument_name>:<quantity>,...] Must have 2-4 distinct legs for combinations.  | [required] |
**delay** | [**Delay**](.md) | Minimum interval between feeds | [required] |

### Return type

[**models::TickerResponse**](TickerResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

