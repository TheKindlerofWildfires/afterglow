local afterglow = Proto("custom", "Afterglow")

local data_seq_no = ProtoField.uint16("afterglow.data.seq_no", "Sequence Number", base.HEX)
local data_msg_no = ProtoField.uint16("afterglow.data.msg_no", "Message Number", base.HEX)
local data_pkt_type = ProtoField.uint16("afterglow.data.data_pkt_type", "Data Type", base.HEX, {
    [0] = "Middle",
    [0x4000] = "Last",
    [0x8000] = "First",
    [0xC000] = "Solo",
})
local data_pkt_order = ProtoField.uint16("afterglow.data.data_pkt_type", "Data Ordered", base.HEX, {
    [0] = "False",
    [0x2000] = "True",
})
local data_time = ProtoField.uint16("afterglow.data.time", "Relative Timestamp", base.HEX)
local data_socket = ProtoField.uint16("afterglow.data.socket", "Destination Socket", base.HEX)
local data_data = ProtoField.bytes("afterglow.data.data", "Data")

-- Define fields for control branch
local control_type = ProtoField.uint16("afterglow.control.type", "Control Type", base.HEX, {
    [0] = "Handshake",
    [1] = "KeepAlive",
    [2] = "Ack",
    [3] = "Loss",
    [4] = "Congestion",
    [5] = "Shutdown",
    [6] = "AckSquare",
    [7] = "Drop",
    [8] = "Err",
    [9] = "Discover",
})
local control_meta_other = ProtoField.uint16("afterglow.control.meta_other", "Meta Other", base.HEX)
local control_meta_seq_no = ProtoField.uint16("afterglow.control.meta_seq_no", "Meta Sequence", base.HEX)
local control_meta_msg_no = ProtoField.uint16("afterglow.control.meta_msg_no", "Meta Message", base.HEX)
local control_time = ProtoField.uint16("afterglow.control.time", "Relative Timestamp", base.HEX)
local control_socket = ProtoField.uint16("afterglow.control.socket", "Destination Socket", base.HEX)

-- Define fields for handshake
local control_handshake_isn = ProtoField.uint16("afterglow.control.handshake_isn", "Initial Sequence Number", base.HEX)
local control_handshake_req_type = ProtoField.uint16("afterglow.control.handshake_req_type", "Handshake Type", base.HEX, {
    [0x8000] = "Connection",
    [0] = "Response",
})
local control_handshake_mss = ProtoField.uint16("afterglow.control.handshake_mss", "MSS", base.HEX)
local control_handshake_flow_control = ProtoField.uint16("afterglow.control.handshake_flow_control", "Flow Control", base.HEX)
local control_handshake_src_socket = ProtoField.uint16("afterglow.control.handshake_src_socket", "Source Socket", base.HEX)
local control_handshake_cookie = ProtoField.uint16("afterglow.control.handshake_cookie", "Cookie", base.HEX)
local control_handshake_port = ProtoField.uint16("afterglow.control.handshake_port", "Return Port", base.DEC)

-- Define fields for ack
local control_ack_seq_no = ProtoField.uint16("afterglow.control.ack_seq_no", "Sequence Number", base.HEX)
local control_ack_rtt = ProtoField.uint16("afterglow.control.ack_rtt", "RTT", base.HEX)
local control_ack_rtt_var = ProtoField.uint16("afterglow.control.ack_rtt_var", "RTT Variance", base.HEX)
local control_ack_buffer_size = ProtoField.uint16("afterglow.control.ack_buffer_size", "Buffer size", base.HEX)
local control_ack_window = ProtoField.uint16("afterglow.control.ack_window", "Window", base.HEX)
local control_ack_bandwidth = ProtoField.uint16("afterglow.control.ack_bandwidth", "Bandwidth", base.HEX)

-- Define fields for loss
local control_loss_start= ProtoField.uint16("afterglow.control.loss_start", "Loss Start", base.HEX)
local control_loss_stop= ProtoField.uint16("afterglow.control.loss_stop", "Loss Stop", base.HEX)

-- Define fields for drop
local control_drop_start= ProtoField.uint16("afterglow.control.drop_start", "Drop Start", base.HEX)
local control_drop_stop= ProtoField.uint16("afterglow.control.drop_stop", "Drop Stop", base.HEX)

-- Define fields for discovery
local control_discovery_data = ProtoField.bytes("afterglow.control.discovery_data", "Data")




-- Register fields in the protocol
afterglow.fields = {
    data_seq_no,
    data_msg_no,
    data_pkt_type,
    data_pkt_order,
    data_time,
    data_socket,
    data_data,
    control_type,
    control_meta_other,
    control_meta_seq_no,
    control_meta_msg_no,
    control_time,
    control_socket,
    control_handshake_isn,
    control_handshake_req_type,
    control_handshake_mss,
    control_handshake_flow_control,
    control_handshake_src_socket,
    control_handshake_cookie,
    control_handshake_port,
    control_ack_seq_no,
    control_ack_rtt,
    control_ack_rtt_var,
    control_ack_buffer_size,
    control_ack_window,
    control_ack_bandwidth,
    control_loss_start,
    control_loss_stop,
    control_discovery_data,
}

function afterglow.dissector(buffer, pinfo, tree)
    pinfo.cols.protocol = "Afterglow"
    -- Check if the packet is at least 1 byte long
    if buffer:len() < 8 then
        return
    end

    -- Get the first bit
    local first_bit = buffer(0,1):bitfield(0,1)

    if first_bit == 0 then
        -- Handle "data" branch
        local data_subtree = tree:add(afterglow, buffer(), "Data")
        local data_type = buffer(2,2):uint()&0xC000
        local data_order = buffer(2,2):uint()&0x2000
        local data_msg = buffer(2,2):uint()&0x1fff
        
        data_subtree:add(data_seq_no, buffer(0,2))
        data_subtree:add(data_msg_no,data_msg)
        data_subtree:add(data_pkt_type,data_type)
        data_subtree:add(data_pkt_order,data_order)
        -- Time handling
        data_subtree:add(data_time, buffer(4,2))

        -- Socket handling
        data_subtree:add(data_socket, buffer(6,2))
        data_subtree:add(data_data, buffer(8,buffer:len()-8))
    else
        -- Handle "control" branch
        local control_subtree = tree:add(afterglow, buffer(), "Control")
        
        -- Figure out the control type
        local control_type_data = buffer(0,2):uint()&0x7fff
        control_subtree:add(control_type, control_type_data)

        -- Meta data handling
        if control_type_data==0 or control_type_data==1 or control_type_data==4 or control_type_data==5 or control_type_data==8 or control_type_data==9 then 
            control_subtree:add(control_meta_other, buffer(2,2))
        elseif control_type_data == 2 or control_type_data == 3 or control_type_data == 6 then
            control_subtree:add(control_meta_seq_no, buffer(2,2))
        else 
            control_subtree:add(control_meta_msg_no, buffer(2,2))
        end

        -- Time handling
        control_subtree:add(control_time, buffer(4,2))

        -- Socket handling
        control_subtree:add(control_socket, buffer(6,2))

        -- Handshake handling
        if control_type_data==0 then
            local handshake_subtree = control_subtree:add(afterglow, buffer(), "Handshake")

            local isn = buffer(8,2):uint()&0x7fff
            local req_type = buffer(8,2):uint()&0x8000
            handshake_subtree:add(control_handshake_isn, isn)
            handshake_subtree:add(control_handshake_req_type, req_type)
            handshake_subtree:add(control_handshake_mss, buffer(10,2))
            handshake_subtree:add(control_handshake_flow_control, buffer(12,2))
            handshake_subtree:add(control_handshake_src_socket, buffer(14,2))
            handshake_subtree:add(control_handshake_cookie, buffer(16,2))
            handshake_subtree:add(control_handshake_port, buffer(18,2))
        elseif control_type_data==2 then
            local ack_subtree = control_subtree:add(afterglow, buffer(), "Ack")
            ack_subtree:add(control_ack_seq_no, buffer(8,2))
            ack_subtree:add(control_ack_rtt, buffer(10,2))
            ack_subtree:add(control_ack_rtt_var, buffer(12,2))
            ack_subtree:add(control_ack_buffer_size, buffer(14,2))
            ack_subtree:add(control_ack_window, buffer(16,2))
            ack_subtree:add(control_ack_bandwidth, buffer(18,2))
        elseif control_type_data==2 then
            local loss_subtree = control_subtree:add(afterglow, buffer(), "Loss")
            loss_subtree:add(control_loss_start, buffer(8,2))
            loss_subtree:add(control_loss_stop, buffer(10,2))
        elseif control_type_data==7 then
            local drop_subtree = control_subtree:add(afterglow, buffer(), "Drop")
            drop_subtree:add(control_loss_start, buffer(8,2))
            drop_subtree:add(control_loss_stop, buffer(10,2))    
        elseif control_type_data==9 then
            local discovery_subtree = control_subtree:add(afterglow, buffer(), "Discovery")
            discovery_subtree:add(control_discovery_data, buffer(8,buffer:len()-8))
        end 
    end
end

-- Register the protocol with UDP port
local udp_port_1 = DissectorTable.get("udp.port")
udp_port_1:add(8128, afterglow)

local udp_port_2 = DissectorTable.get("udp.port")
udp_port_2:add(8192, afterglow)