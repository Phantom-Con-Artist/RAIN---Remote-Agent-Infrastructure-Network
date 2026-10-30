use crate::ChatStore;
use rain_protocol::{ChatMessage, MessageDeliveryStatus, SenderType};
use serde_json::{json, Value};
use uuid::Uuid;
fn suggestion(name: &str, description: &str, kind: &str) -> Value {
    json!({"name":name,"description":description,"kind":kind,"insert":format!("{name} ")})
}
fn channel_commands() -> Vec<Value> {
    vec![
        suggestion("/help", "Show channel commands", "rain"),
        suggestion(
            "/pause",
            "Pause this channel and cancel pending turns",
            "rain",
        ),
        suggestion("/resume", "Reopen this conversation", "rain"),
        suggestion(
            "/stop",
            "Stop autonomous discussion in this channel",
            "rain",
        ),
        suggestion(
            "/autonomy on",
            "Enable open-ended channel discussion",
            "rain",
        ),
        suggestion(
            "/autonomy off",
            "Only human messages invite responses",
            "rain",
        ),
    ]
}
impl ChatStore {
    pub async fn command_catalog(&self, id: Uuid) -> Result<Value, String> {
        let c = self
            .conversations
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or("Conversation missing")?;
        if c.is_group {
            return Ok(
                json!({"scope":"channel","commands":channel_commands(),"efforts":[],"native_slash":false}),
            );
        }
        let mut commands = vec![
            suggestion("/help", "Commands available in this direct chat", "rain"),
            suggestion(
                "/status",
                "Show agent, machine and direct-chat settings",
                "rain",
            ),
            suggestion(
                "/reset",
                "Start a fresh provider thread; keep RAIN history",
                "rain",
            ),
            suggestion("/cancel", "Cancel this agent's current turn", "rain"),
            suggestion("/cli-help", "Show this installed CLI's help", "native"),
        ];
        let source = self
            .supervisor
            .list_agents()
            .await
            .into_iter()
            .find(|a| c.participant_ids.contains(&a.id.to_string()))
            .ok_or("Agent session is unavailable")?;
        if !source.provider.ends_with("_chat") {
            return Ok(
                json!({"scope":"terminal","commands":[],"efforts":[],"native_slash":true,"help":"Use the embedded native terminal for this session"}),
            );
        }
        let cached = self
            .native_catalogs
            .read()
            .await
            .get(&source.id.to_string())
            .cloned();
        let capabilities = if let Some(value) = cached {
            value
        } else {
            let result=crate::host_access::operate(source.machine.as_deref(),json!({"action":"cli_catalog","command":source.command,"provider":source.provider,"cwd":source.cwd})).await.unwrap_or(json!({"supports_model":false,"efforts":[],"commands":[],"help":"The CLI help could not be read. Use its native terminal for the complete interface."}));
            self.native_catalogs
                .write()
                .await
                .insert(source.id.to_string(), result.clone());
            result
        };
        if capabilities["supports_model"] == true {
            commands.push(suggestion(
                "/model",
                "Set a model for this direct thread: /model <id>",
                "setting",
            ));
        }
        if capabilities["efforts"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false)
        {
            commands.push(suggestion(
                "/effort",
                "Set reasoning effort for this direct thread",
                "setting",
            ));
        }
        for name in capabilities["commands"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str())
        {
            commands.push(suggestion(
                &format!("/{name}"),
                "Run this installed provider's CLI command",
                "native",
            ));
        }
        Ok(
            json!({"scope":"direct","provider":source.provider,"commands":commands,"efforts":capabilities["efforts"],"native_slash":capabilities["native_slash"],"settings":c.native_settings,"help":capabilities["help"]}),
        )
    }
    pub async fn native_terminal(&self, id: Uuid) -> Result<rain_protocol::AgentProcess, String> {
        let c = self
            .conversations
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or("Conversation missing")?;
        if c.is_group {
            return Err("Provider terminals are opened from private direct chats".into());
        }
        if let Some(id) = c.native_terminal_id {
            if let Some(agent) = self.supervisor.get_agent(id).await {
                if agent.status == rain_protocol::AgentStatus::Running {
                    return Ok(agent);
                }
            }
        }
        let source = self
            .supervisor
            .list_agents()
            .await
            .into_iter()
            .find(|a| c.participant_ids.contains(&a.id.to_string()))
            .ok_or("Agent session unavailable")?;
        let mut args = Vec::new();
        if let Some(model) = c.native_settings.model {
            args.extend(["--model".into(), model]);
        }
        if let Some(effort) = c.native_settings.effort {
            args.extend([
                if source.provider == "muse_chat" {
                    "--reasoning-effort"
                } else {
                    "--effort"
                }
                .into(),
                effort,
            ]);
        }
        let terminal = self
            .supervisor
            .start_agent(rain_protocol::AgentLaunchRequest {
                name: format!("{} native CLI", source.name),
                provider: "custom".into(),
                command: source.command,
                args,
                cwd: Some(source.cwd),
                machine: source.machine,
                interactive: true,
                env_vars: Default::default(),
            })
            .await?;
        if let Some(c) = self.conversations.write().await.get_mut(&id) {
            c.native_terminal_id = Some(terminal.id);
        }
        Ok(terminal)
    }
    pub(crate) async fn direct_command(
        &self,
        id: Uuid,
        text: &str,
    ) -> Option<Result<ChatMessage, String>> {
        let text = text.trim();
        let token = text.split_whitespace().next().unwrap_or("");
        let Some(command) = token.strip_prefix('/') else {
            return None;
        };
        if command.is_empty()
            || !command
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return None;
        }
        let argument = text[token.len()..].trim();
        let c = match self.conversations.read().await.get(&id).cloned() {
            Some(c) => c,
            None => return Some(Err("Conversation missing".into())),
        };
        if c.is_group {
            if command == "help" {
                return Some(Ok(self
                    .command_result(
                        id,
                        text,
                        &channel_commands()
                            .iter()
                            .map(|c| {
                                format!(
                                    "{} — {}",
                                    c["name"].as_str().unwrap_or_default(),
                                    c["description"].as_str().unwrap_or_default()
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n"),
                    )
                    .await));
            }
            return Some(Err("Provider/model commands belong in a private direct chat. Channel commands are /help, /pause, /resume, /stop and /autonomy on|off.".into()));
        }
        if self
            .supervisor
            .list_agents()
            .await
            .into_iter()
            .find(|a| c.participant_ids.contains(&a.id.to_string()))
            .map(|a| !a.provider.ends_with("_chat"))
            .unwrap_or(false)
        {
            return None;
        }
        let catalog = match self.command_catalog(id).await {
            Ok(c) => c,
            Err(e) => return Some(Err(e)),
        };
        let agent = match self
            .supervisor
            .list_agents()
            .await
            .into_iter()
            .find(|a| c.participant_ids.contains(&a.id.to_string()))
        {
            Some(a) => a,
            None => return Some(Err("Agent session unavailable".into())),
        };
        let result=async {
   let output=match command {
    "help"=>catalog["commands"].as_array().unwrap().iter().map(|c|format!("{} — {}",c["name"].as_str().unwrap_or_default(),c["description"].as_str().unwrap_or_default())).collect::<Vec<_>>().join("\n"),
    "cli-help"=>catalog["help"].as_str().unwrap_or("Native help is unavailable").into(),
    "status"=>format!("{} · {}\nMachine: {}\nWorkspace: {}\nModel override: {}\nEffort override: {}\nProvider threads and settings are scoped to this direct chat.",agent.name,agent.provider,agent.machine.as_deref().unwrap_or("This host"),agent.cwd,c.native_settings.model.as_deref().unwrap_or("Native CLI default"),c.native_settings.effort.as_deref().unwrap_or("Native CLI default")),
    "model"=>{
     if !catalog["commands"].as_array().unwrap().iter().any(|v|v["name"]=="/model"){return Err("This installed adapter does not expose a --model option. Use its native terminal.".into());}
     if argument.is_empty(){format!("Current override: {}. Use /model <model-id> or /model default. Model IDs and availability are determined by your provider.",c.native_settings.model.as_deref().unwrap_or("Native CLI default"))}
     else{
      if argument.len()>256||!argument.chars().all(|c|c.is_ascii_alphanumeric()||"_-.:/".contains(c))||argument.starts_with('-'){return Err("Enter one model ID without CLI flags or spaces".into());}
      let model=if argument=="default"{None}else{Some(argument.to_string())};self.conversations.write().await.get_mut(&id).ok_or("Conversation missing")?.native_settings.model=model;
      format!("Model override set to {argument} for this direct chat. It applies on the next turn; the provider validates the model ID.")
     }
    },
    "effort"=>{
     let efforts=catalog["efforts"].as_array().ok_or("This adapter has no effort setting")?;
     if argument.is_empty(){format!("Current effort: {}. Available: {}",c.native_settings.effort.as_deref().unwrap_or("Native default"),efforts.iter().filter_map(|v|v.as_str()).collect::<Vec<_>>().join(", "))}
     else{
      if argument!="default"&&!efforts.iter().any(|v|v.as_str()==Some(argument)){return Err("That effort level is not exposed by this installed provider".into());}
      self.conversations.write().await.get_mut(&id).ok_or("Conversation missing")?.native_settings.effort=if argument=="default"{None}else{Some(argument.into())};
      format!("Reasoning effort set to {argument} for the next turn in this direct chat.")
     }
    },
    "cancel"=>{self.invalidate_human_deliveries(id).await;self.supervisor.cancel_chat_conversation(agent.id,id).await?;"Current direct-chat turn cancelled. The session and history remain available.".into()},
    "reset"|"clear"=>{self.invalidate_human_deliveries(id).await;self.supervisor.send_raw(agent.id,&format!("{}\n",json!({"control":"reset","conversation_id":id}))).await?;"Provider thread reset. RAIN history and membership-scoped group awareness are preserved.".into()},
    _=>{
     let available=catalog["commands"].as_array().unwrap().iter().any(|v|v["name"].as_str()==Some(token)&&v["kind"]=="native");
     if !available{return Err("This command is not exposed by the installed headless adapter. Open Native terminal for its full CLI interface.".into());}
     let response=crate::host_access::operate_timeout(agent.machine.as_deref(),json!({"action":"cli_command","command":agent.command,"provider":agent.provider,"cwd":agent.cwd,"subcommand":command,"arguments":argument}),60).await?;
     response["output"].as_str().unwrap_or("Command completed without output").to_string()
    },
   };
   Ok::<_,String>(self.command_result(id,text,&output).await)
  }.await;
        if result
            .as_ref()
            .err()
            .map(|_| {
                catalog["native_slash"] == true
                    && !matches!(
                        command,
                        "model" | "effort" | "cancel" | "reset" | "clear" | "cli-help" | "status"
                    )
            })
            .unwrap_or(false)
        {
            return None;
        }
        Some(result)
    }
    async fn command_result(&self, id: Uuid, command: &str, output: &str) -> ChatMessage {
        let now = chrono::Utc::now();
        let human = ChatMessage {
            id: Uuid::new_v4(),
            conversation_id: id,
            sender_id: "human-user".into(),
            sender_name: "You".into(),
            sender_type: SenderType::Human,
            text: command.into(),
            status: MessageDeliveryStatus::Delivered,
            attachments: vec![],
            job_delegation: None,
            coworker: None,
            timestamp: now,
        };
        let response = ChatMessage {
            id: Uuid::new_v4(),
            conversation_id: id,
            sender_id: "system".into(),
            sender_name: "RAIN · CLI".into(),
            sender_type: SenderType::System,
            text: output.chars().take(33000).collect(),
            status: MessageDeliveryStatus::Delivered,
            attachments: vec![],
            job_delegation: None,
            coworker: None,
            timestamp: now,
        };
        let mut conversations = self.conversations.write().await;
        if let Some(c) = conversations.get_mut(&id) {
            c.last_message = Some(format!("CLI: {command}"));
            c.last_message_time = Some(now);
            let mut messages = self.messages.write().await;
            let list = messages.entry(id).or_default();
            list.push(human);
            list.push(response.clone());
        }
        response
    }
}
