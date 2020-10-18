local M = {}

local socket = require("socket")
local messagepack = require("lua/common/libs/Lua-MessagePack/MessagePack")

local connection = {
  tcp = nil,
  connected = false,
  client_id = 0,
  heartbeat_time = 1,
  timer = 0,
  timeout_buffer = nil
}

local MESSAGETYPE_TRANSFORM = 0
local MESSAGETYPE_VEHICLE_SPAWN = 1
local MESSAGETYPE_ELECTRICS = 2
local MESSAGETYPE_GEARBOX = 3
local MESSAGETYPE_NODES = 4
local MESSAGETYPE_VEHICLE_REMOVE = 5
local MESSAGETYPE_VEHICLE_RESET = 6
local MESSAGETYPE_CLIENT_INFO= 7
local MESSAGETYPE_CHAT = 8

local function send_data(data_type, reliable, data)
  if not connection.connected then return -1 end
  local len = #data
  local len = ffi.string(ffi.new("uint32_t[?]", 1, {len}), 4)
  if reliable then
    reliable = 1
  else
    reliable = 0
  end
  connection.tcp:send(string.char(reliable)..string.char(data_type)..len)
  connection.tcp:send(data)
end

local function connect(addr, player_name)
  print("Connecting...")
  kissui.add_message("Connecting to "..addr.."...")
  connection.tcp = socket.tcp()
  connection.tcp:settimeout(5.0)
  local connected, err = connection.tcp:connect("127.0.0.1", "7894")

  -- Send server address to the bridge
  local addr_lenght = ffi.string(ffi.new("uint32_t[?]", 1, {#addr}), 4)
  connection.tcp:send(addr_lenght)
  connection.tcp:send(addr)

  local _ = connection.tcp:receive(1)
  local len, _, _ = connection.tcp:receive(4)
  local len = ffi.cast("uint32_t*", ffi.new("char[?]", #len, len))
  local len = len[0]

  local received, _, _ = connection.tcp:receive(len)
  local server_info = jsonDecode(received)
  if not server_info then
    print("Failed to fetch server info")
    return
  end
  print("Server name: "..server_info.name)
  print("Player count: "..server_info.player_count)

  connection.tcp:settimeout(0.0)
  connection.connected = true
  connection.client_id = server_info.client_id
  kissui.add_message("Connected!");
  local client_info = {
    name = player_name
  }
  send_data(MESSAGETYPE_CLIENT_INFO, true, jsonEncode(client_info))
  if not server_info.map == "any" then
    freeroam_freeroam.startFreeroam(server_info.map)
  end
  if be:getPlayerVehicle(0) then
    vehiclemanager.send_vehicle_config(be:getPlayerVehicle(0):getID())
  end
end

local function send_messagepack(data_type, reliable, data)
  local data = messagepack.pack(jsonDecode(data))
  send_data(data_type, reliable, data)
end

local function onUpdate(dt)
  if not connection.connected then return end

  if connection.timer < connection.heartbeat_time then
    connection.timer = connection.timer + dt
  else
    connection.timer = 0
    send_data(254, false, "hi")
  end

  while true do
    local received, _, _ = connection.tcp:receive(1)
    if not received then break end
    connection.tcp:settimeout(5.0)
    local data_type = string.byte(received)

    local data = connection.tcp:receive(4)
    local len = ffi.cast("uint32_t*", ffi.new("char[?]", 4, data))

    local data, _, _ = connection.tcp:receive(len[0])

    connection.tcp:settimeout(0.0)

    if data_type == MESSAGETYPE_TRANSFORM then
      local p = ffi.new("char[?]", #data, data)
      local ptr = ffi.cast("float*", p)
      local transform = {}
      transform.position = {ptr[0], ptr[1], ptr[2]}
      transform.rotation = {ptr[3], ptr[4], ptr[5], ptr[6]}
      transform.velocity = {ptr[7], ptr[8], ptr[9]}
      transform.angular_velocity = {ptr[10], ptr[11], ptr[12]}
      transform.owner = ptr[13]
      transform.generation = ptr[14]
      kisstransform.update_vehicle_transform(transform)
    elseif data_type == MESSAGETYPE_VEHICLE_SPAWN then
      local decoded = jsonDecode(data)
      vehiclemanager.spawn_vehicle(decoded)
    elseif data_type == MESSAGETYPE_ELECTRICS then
      vehiclemanager.update_vehicle_electrics(data)
    elseif data_type == MESSAGETYPE_GEARBOX then
      vehiclemanager.update_vehicle_gearbox(data)
    elseif data_type == MESSAGETYPE_NODES then
      vehiclemanager.update_vehicle_nodes(data)
    elseif data_type == MESSAGETYPE_VEHICLE_REMOVE then
      vehiclemanager.remove_vehicle(ffi.cast("uint32_t*", ffi.new("char[?]", 4, data))[0])
    elseif data_type == MESSAGETYPE_VEHICLE_RESET then
      vehiclemanager.reset_vehicle(ffi.cast("uint32_t*", ffi.new("char[?]", 4, data))[0])
    elseif data_type == MESSAGETYPE_CHAT then
      kissui.add_message(data)
    end
  end
end

local function get_client_id()
  return connection.client_id
end

M.get_client_id = get_client_id
M.connect = connect
M.send_data = send_data
M.onUpdate = onUpdate
M.send_messagepack = send_messagepack

return M
