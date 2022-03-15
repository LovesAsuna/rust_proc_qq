use crate::tools::ReplyChain;
use lazy_static::lazy_static;
use proc_qq::re_exports::rq_engine::msg::elem::RQElem;
use proc_qq::re_exports::rs_qq::msg::elem::At;
use proc_qq::re_exports::rs_qq::msg::MessageChain;
use proc_qq::{
    event, module, ClientTrait, GroupTrait, MemberTrait, MessageChainTrait, MessageContentTrait,
    MessageEvent, MessageSendToSourceTrait, Module, TextEleParseTrait,
};
use regex::Regex;
use std::time::Duration;

static ID: &'static str = "group_admin";
static NAME: &'static str = "群管";

lazy_static! {
    static ref BAN_REGEXP: Regex =
        Regex::new("^(\\s+)?b(\\s+)?([0-9]{1,5})(\\s+)?([smhd]?)(\\s+)?").unwrap();
}

pub fn module() -> Module {
    module!(ID, NAME, on_message)
}

#[event]
async fn on_message(event: &MessageEvent) -> anyhow::Result<bool> {
    let content = event.message_content();
    if content == NAME {
        if !event.is_group_message() {
            event
                .send_message_to_source(
                    event
                        .reply_chain()
                        .await
                        .append("只能在群中使用".parse_text()),
                )
                .await?;
        } else {
            event
                .send_message_to_source(
                    event.reply_chain().await.append(
                        ("".to_owned()
                            + "b+禁言时间 @一个或多个人\n\n"
                            + "比如禁言张三12小时 : b12h @张三 \n\n"
                            + "比如禁言张三李四12天 : b12h @张三 @李四 \n\n"
                            + " s 秒, m 分, h 小时, d 天\n\n"
                            + "b0 则解除禁言")
                            .parse_text(),
                    ),
                )
                .await?;
        }
        return Ok(true);
    }
    if !event.is_group_message() {
        return Ok(false);
    }
    let group_message = event.as_group_message()?;
    if BAN_REGEXP.is_match(&content) {
        let group = group_message
            .must_find_group(group_message.message.group_code, true)
            .await?;
        let call_member = group.must_find_member(event.from_uin()).await?;
        let bot_member = group.must_find_member(event.bot_uin().await).await?;
        if call_member.is_member() {
            group_message
                .send_message_to_source(
                    group_message
                        .reply_chain()
                        .await
                        .append("您必须是群主或管理员才能使用".parse_text()),
                )
                .await?;
            return Ok(true);
        }
        if bot_member.is_member() {
            let mut chan = MessageChain::default();
            chan.push(At::new(group_message.message.from_uin));
            chan.push("\n\n机器人必须是群主或管理员才能使用".parse_text());
            group_message.send_message_to_source(chan).await?;
            return Ok(true);
        }
        let e = BAN_REGEXP.captures_iter(&content).next().unwrap();
        let mut time = e.get(3).unwrap().as_str().parse::<u64>()?;
        time = match e.get(5).unwrap().as_str() {
            "m" => time * 60,
            "h" => time * 60 * 60,
            "d" => time * 60 * 60 * 24,
            _ => time,
        };
        if time >= 60 * 60 * 24 * 29 {
            let mut chan = MessageChain::default();
            chan.push(At::new(group_message.message.from_uin));
            chan.push("\n\n最多禁言29天".parse_text());
            group_message.send_message_to_source(chan).await?;
            return Ok(true);
        }
        for x in group_message.message.elements.clone().into_iter() {
            match x {
                RQElem::At(id) => {
                    event
                        .client()
                        .group_mute(
                            group_message.message.group_code,
                            id.target,
                            Duration::from_secs(time),
                        )
                        .await?;
                }
                _ => (),
            }
        }
        let mut chan = MessageChain::default();
        chan.push(At::new(group_message.message.from_uin));
        chan.push("\n\nOK".parse_text());
        group_message.send_message_to_source(chan).await?;
        return Ok(true);
    }
    Ok(false)
}