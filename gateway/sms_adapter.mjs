/**
 * sms_adapter.mjs — pluggable SMS transport for phone OTP.
 *
 * ABORN_SMS_SEND=aliyun   → Aliyun Cloud SMS (requires @alicloud/pop-core)
 * ABORN_SMS_SEND=tencent  → Tencent Cloud SMS (requires tencentcloud-sdk-nodejs)
 * (unset)                 → dev console fallback
 */

/** Aliyun Cloud SMS sender. */
export function createAliyunSms({ accessKeyId, accessKeySecret, signName, templateCode }) {
  if (!accessKeyId || !accessKeySecret) throw new Error('Aliyun SMS: missing accessKeyId/accessKeySecret')
  return async function sendAliyun(phone, otp) {
    try {
      const Client = (await import('@alicloud/pop-core')).default
      const client = new Client({
        accessKeyId, accessKeySecret,
        endpoint: 'https://dysmsapi.aliyuncs.com',
        apiVersion: '2017-05-25',
      })
      await client.request('SendSms', {
        PhoneNumbers:  phone,
        SignName:      signName,
        TemplateCode:  templateCode,
        TemplateParam: JSON.stringify({ code: otp }),
      }, { method: 'POST' })
    } catch (e) {
      throw new Error('AliyunSmsDeliveryError: ' + e.message)
    }
  }
}

/** Tencent Cloud SMS sender. */
export function createTencentSms({ secretId, secretKey, sdkAppId, signName, templateId }) {
  if (!secretId || !secretKey) throw new Error('Tencent SMS: missing secretId/secretKey')
  return async function sendTencent(phone, otp) {
    try {
      const { default: tc } = await import('tencentcloud-sdk-nodejs')
      const client = new tc.sms.v20210111.Client({
        credential: { secretId, secretKey },
        region: 'ap-guangzhou',
        profile: { httpProfile: { endpoint: 'sms.tencentcloudapi.com' } },
      })
      await client.SendSms({
        SmsSdkAppId:      sdkAppId,
        SignName:          signName,
        TemplateId:        templateId,
        TemplateParamSet:  [otp],
        PhoneNumberSet:    [phone.startsWith('+') ? phone : '+86' + phone],
      })
    } catch (e) {
      throw new Error('TencentSmsDeliveryError: ' + e.message)
    }
  }
}

/** Test mock sender — captures all sent messages. */
export function createMockSms() {
  const sent = []
  const fn = async (phone, otp) => { sent.push({ phone, otp }) }
  fn.sent    = sent
  fn.clear   = () => { sent.length = 0 }
  fn.lastOtp = () => sent.at(-1)?.otp ?? null
  return fn
}

/** Auto-detect sender from environment variables. */
export function createSmsSender() {
  const mode = process.env.ABORN_SMS_SEND || 'console'
  if (mode === 'aliyun') {
    return createAliyunSms({
      accessKeyId:     process.env.ABORN_ALIYUN_KEY_ID    || '',
      accessKeySecret: process.env.ABORN_ALIYUN_KEY_SECRET || '',
      signName:        process.env.ABORN_ALIYUN_SIGN       || 'aborn',
      templateCode:    process.env.ABORN_ALIYUN_TEMPLATE   || '',
    })
  }
  if (mode === 'tencent') {
    return createTencentSms({
      secretId:   process.env.ABORN_TENCENT_SECRET_ID  || '',
      secretKey:  process.env.ABORN_TENCENT_SECRET_KEY || '',
      sdkAppId:   process.env.ABORN_TENCENT_APP_ID     || '',
      signName:   process.env.ABORN_TENCENT_SIGN        || 'aborn',
      templateId: process.env.ABORN_TENCENT_TEMPLATE    || '',
    })
  }
  // Dev fallback: print to console
  return async (phone, otp) => {
    console.log(`[sms:dev] OTP for ${phone}: ${otp}`)
  }
}
